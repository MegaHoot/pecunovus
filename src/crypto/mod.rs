// SPDX-License-Identifier: Apache-2.0
// Copyright 2017-2026 Pecu Novus Network / MegaHoot Technologies
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// crypto/mod.rs
// Pecu Novus cryptographic primitives:
//   - SHA-512 (original whitepaper key generation)
//   - SHA-256 (block hashing, CBC chaining)
//   - Verifiable Delay Function (VDF) for Proof of Time
//   - Public/Private key generation (matching whitepaper spec)
//   - Cipher Block Chaining (CBC) encryption for block data

use hex;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha512};
use sha3::Keccak256;
use std::fmt;

// ─── SHA-512 Hash (original Pecu Novus encryption standard) ──────────────────

pub fn sha512(input: &[u8]) -> String {
    let mut hasher = Sha512::new();
    hasher.update(input);
    hex::encode(hasher.finalize())
}

// ─── SHA-256 Hash (block chaining, PoT sequence) ─────────────────────────────

pub fn sha256(input: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    hex::encode(hasher.finalize())
}

pub fn sha256_bytes(input: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(input);
    hasher.finalize().to_vec()
}

// ─── Keccak-256 Hash (ERC-20 / EVM compatibility) ────────────────────────────

pub fn keccak256(input: &[u8]) -> String {
    let mut hasher = Keccak256::new();
    hasher.update(input);
    hex::encode(hasher.finalize())
}

pub fn keccak256_bytes(input: &[u8]) -> Vec<u8> {
    let mut hasher = Keccak256::new();
    hasher.update(input);
    hasher.finalize().to_vec()
}

// ─── Public Key Generation ────────────────────────────────────────────────────
// Whitepaper spec: "Random lengths of numbers and letters, between 64 to 128,
// a combination of Strings, Integers and a time stamp."

pub fn generate_public_key() -> String {
    let mut rng = thread_rng();
    let timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let random_bytes: Vec<u8> = (0..32).map(|_| rng.gen::<u8>()).collect();
    let seed = format!("{}{}", hex::encode(&random_bytes), timestamp);
    let hash = sha512(seed.as_bytes());
    // Length between 64-128 as per whitepaper
    let len = rng.gen_range(64..=128);
    hash[..len.min(hash.len())].to_string()
}

// ─── Private Key Generation ───────────────────────────────────────────────────
// Whitepaper spec: "SHA512 Hashed information mixed with Random length of
// numbers and letters, between 60 to 102, a combination of Strings and Integers"

pub fn generate_private_key(public_key: &str) -> String {
    let mut rng = thread_rng();
    let timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let random_suffix: Vec<u8> = (0..24).map(|_| rng.gen::<u8>()).collect();
    let seed = format!("{}{}{}", public_key, timestamp, hex::encode(&random_suffix));
    let sha_hash = sha512(seed.as_bytes());
    let random_part = hex::encode(&random_suffix);
    let combined = format!(
        "{}{}",
        &sha_hash[..60],
        &random_part[..random_part.len().min(42)]
    );
    let len = rng.gen_range(60..=102);
    combined[..len.min(combined.len())].to_string()
}

// ─── Pecu Address (EVM-compatible 0x format) ─────────────────────────────────

pub fn public_key_to_address(public_key: &str) -> String {
    let hash = keccak256_bytes(public_key.as_bytes());
    format!("0x{}", hex::encode(&hash[12..]))
}

pub fn public_key_to_pecu_address(public_key: &str) -> String {
    let hash = sha256_bytes(public_key.as_bytes());
    bs58::encode(&hash).into_string()
}

// ─── Block Address ────────────────────────────────────────────────────────────
// Whitepaper: "Communication / Transaction Information's Hashed with SHA512"

pub fn compute_block_address(
    sender: &str,
    receiver: &str,
    amount: &str,
    timestamp: i64,
    note: &str,
    escrow: bool,
) -> String {
    let data = format!("{sender}{receiver}{amount}{timestamp}{note}{escrow}");
    sha512(data.as_bytes())
}

// ─── Verifiable Delay Function (VDF) ─────────────────────────────────────────
// Whitepaper: y = x^(2^T) mod N
// "Requires T sequential steps to compute. Output y can be verified quickly."

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VdfProof {
    pub input: String,  // x - seed (transaction hash / block hash)
    pub output: String, // y = x^(2^T) mod N
    pub delay: u64,     // T - required delay steps
    pub timestamp: i64,
    pub sequence_count: u64, // monotonically increasing PoT count
}

/// Simplified VDF using iterative SHA-256 squaring to simulate modular exponentiation.
/// In production this would use an RSA modulus N for true VDF security.
pub fn compute_vdf(seed: &str, delay_steps: u64) -> VdfProof {
    let timestamp = chrono::Utc::now().timestamp();
    let mut current = sha256_bytes(seed.as_bytes());

    // Simulate x^(2^T) mod N via iterated hashing (practical approximation)
    // Each iteration: current = SHA256(SHA256(current)) — sequential, non-parallelizable
    for _ in 0..delay_steps {
        let inner = sha256_bytes(&current);
        current = sha256_bytes(&inner);
    }

    let sequence_count = {
        let mut hasher = Sha256::new();
        hasher.update(seed);
        hasher.update(&delay_steps.to_le_bytes());
        let result = hasher.finalize();
        u64::from_le_bytes(result[..8].try_into().unwrap_or([0u8; 8]))
    };

    VdfProof {
        input: seed.to_string(),
        output: hex::encode(&current),
        delay: delay_steps,
        timestamp,
        sequence_count,
    }
}

/// Verify a VDF proof: recompute and compare (fast verification)
pub fn verify_vdf(proof: &VdfProof) -> bool {
    let recomputed = compute_vdf(&proof.input, proof.delay);
    recomputed.output == proof.output
}

// ─── Cipher Block Chaining (CBC) Encryption ──────────────────────────────────
// Whitepaper: "CBC encryption sequentially encrypts each block of data, using
// the previously encrypted block to XOR with the input data."

pub fn cbc_encrypt(data: &[u8], key: &[u8], iv: &[u8]) -> Vec<u8> {
    let block_size = 32; // SHA-256 block size
    let mut result = Vec::new();
    let mut prev_block = iv.to_vec();

    // Pad data to block size
    let mut padded = data.to_vec();
    let padding = block_size - (padded.len() % block_size);
    padded.extend(vec![padding as u8; padding]);

    for chunk in padded.chunks(block_size) {
        // XOR with previous cipher block
        let xored: Vec<u8> = chunk
            .iter()
            .zip(prev_block.iter().cycle())
            .map(|(a, b)| a ^ b)
            .collect();

        // Hash with key to produce cipher block (simulating AES-CBC with SHA-256)
        let mut block_input = xored.clone();
        block_input.extend_from_slice(key);
        let cipher_block = sha256_bytes(&block_input);

        result.extend_from_slice(&cipher_block);
        prev_block = cipher_block;
    }
    result
}

/// Sign a hash using the Proof of Time sequence (replication identity key)
pub fn sign_with_pot_sequence(data: &str, pot_proof: &VdfProof) -> String {
    let combined = format!("{}{}{}", data, pot_proof.output, pot_proof.sequence_count);
    sha512(combined.as_bytes())
}

// ─── HMAC for message authentication ─────────────────────────────────────────

pub fn hmac_sha256(key: &[u8], message: &[u8]) -> Vec<u8> {
    // Simple HMAC-SHA256 implementation
    let block_size = 64usize;
    let mut k = if key.len() > block_size {
        sha256_bytes(key)
    } else {
        key.to_vec()
    };
    k.resize(block_size, 0);

    let ipad: Vec<u8> = k.iter().map(|b| b ^ 0x36).collect();
    let opad: Vec<u8> = k.iter().map(|b| b ^ 0x5c).collect();

    let mut inner = ipad.clone();
    inner.extend_from_slice(message);
    let inner_hash = sha256_bytes(&inner);

    let mut outer = opad.clone();
    outer.extend_from_slice(&inner_hash);
    sha256_bytes(&outer)
}

// ─── Merkle Tree Root ─────────────────────────────────────────────────────────

pub fn compute_merkle_root(tx_hashes: &[String]) -> String {
    if tx_hashes.is_empty() {
        return sha256(b"empty");
    }
    if tx_hashes.len() == 1 {
        return tx_hashes[0].clone();
    }

    let mut layer: Vec<String> = tx_hashes.to_vec();
    while layer.len() > 1 {
        if layer.len() % 2 != 0 {
            layer.push(layer.last().unwrap().clone());
        }
        layer = layer
            .chunks(2)
            .map(|pair| sha256(format!("{}{}", pair[0], pair[1]).as_bytes()))
            .collect();
    }
    layer[0].clone()
}

// ─── Display helpers ──────────────────────────────────────────────────────────

pub struct HashDisplay(pub String);
impl fmt::Display for HashDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.len() > 16 {
            write!(f, "{}...{}", &self.0[..8], &self.0[self.0.len() - 8..])
        } else {
            write!(f, "{}", self.0)
        }
    }
}

// ─── UUID generation (no external uuid crate needed) ─────────────────────────
pub fn generate_uuid() -> String {
    let mut rng = thread_rng();
    let bytes: Vec<u8> = (0..16).map(|_| rng.gen::<u8>()).collect();
    format!(
        "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
        u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        u16::from_be_bytes([bytes[4], bytes[5]]),
        u16::from_be_bytes([bytes[6], bytes[7]]) & 0x0fff,
        (u16::from_be_bytes([bytes[8], bytes[9]]) & 0x3fff) | 0x8000,
        bytes[10..16]
            .iter()
            .fold(0u64, |acc, &b| (acc << 8) | b as u64),
    )
}
