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

// wallet/mod.rs
// Pecu Novus Wallet - public/private key pairs, addresses (EVM + Pecu native)
// Supports GAK (General Access Key) and DAK (Development Access Key)

use crate::crypto;
use serde::{Deserialize, Serialize};

use chrono::Utc;

// ─── Key Pair ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPair {
    pub public_key: String,
    pub private_key: String,
    /// EVM-compatible address (0x...)
    pub evm_address: String,
    /// Native Pecu address (Base58)
    pub pecu_address: String,
}

impl KeyPair {
    pub fn generate() -> Self {
        let public_key = crypto::generate_public_key();
        let private_key = crypto::generate_private_key(&public_key);
        let evm_address = crypto::public_key_to_address(&public_key);
        let pecu_address = crypto::public_key_to_pecu_address(&public_key);
        KeyPair { public_key, private_key, evm_address, pecu_address }
    }

    pub fn sign(&self, data: &str) -> String {
        let combined = format!("{}{}", self.private_key, data);
        crypto::sha512(combined.as_bytes())
    }

    pub fn verify_signature(&self, data: &str, signature: &str) -> bool {
        self.sign(data) == signature
    }

    /// Refresh public key (Pecu Novus security feature from whitepaper)
    pub fn refresh_public_key(&mut self) {
        let new_pub = crypto::generate_public_key();
        let new_priv = crypto::generate_private_key(&new_pub);
        self.public_key = new_pub;
        self.private_key = new_priv;
        self.evm_address = crypto::public_key_to_address(&self.public_key);
        self.pecu_address = crypto::public_key_to_pecu_address(&self.public_key);
    }
}

// ─── General Access Key (GAK) ────────────────────────────────────────────────
// Whitepaper: "allows Pecu Wallet holders to seamlessly connect and disconnect
// their wallets from applications within the ecosystem."

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralAccessKey {
    pub key_id: String,
    pub wallet_address: String,
    pub app_id: String,
    pub is_connected: bool,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

impl GeneralAccessKey {
    pub fn new(wallet_address: &str, app_id: &str, ttl_seconds: Option<i64>) -> Self {
        let now = Utc::now().timestamp();
        GeneralAccessKey {
            key_id: crate::crypto::generate_uuid(),
            wallet_address: wallet_address.to_string(),
            app_id: app_id.to_string(),
            is_connected: true,
            created_at: now,
            expires_at: ttl_seconds.map(|s| now + s),
        }
    }

    pub fn disconnect(&mut self) {
        self.is_connected = false;
    }

    pub fn is_valid(&self) -> bool {
        if !self.is_connected { return false; }
        if let Some(exp) = self.expires_at {
            return Utc::now().timestamp() < exp;
        }
        true
    }
}

// ─── Development Access Key (DAK) ────────────────────────────────────────────
// Whitepaper: "Every developer must register for a DAK, ensuring their identity
// is known and verified." — KYC-based accountability layer.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentAccessKey {
    pub dak_id: String,
    pub developer_name: String,
    pub developer_email: String,
    pub is_kyc_verified: bool,
    pub is_active: bool,
    pub issued_at: i64,
    pub revoked_at: Option<i64>,
    pub revocation_reason: Option<String>,
}

impl DevelopmentAccessKey {
    pub fn new(developer_name: &str, developer_email: &str) -> Self {
        DevelopmentAccessKey {
            dak_id: crate::crypto::generate_uuid(),
            developer_name: developer_name.to_string(),
            developer_email: developer_email.to_string(),
            is_kyc_verified: false,
            is_active: false,
            issued_at: Utc::now().timestamp(),
            revoked_at: None,
            revocation_reason: None,
        }
    }

    pub fn verify_kyc(&mut self) {
        self.is_kyc_verified = true;
        self.is_active = true;
    }

    pub fn revoke(&mut self, reason: &str) {
        self.is_active = false;
        self.revoked_at = Some(Utc::now().timestamp());
        self.revocation_reason = Some(reason.to_string());
    }

    pub fn is_valid(&self) -> bool {
        self.is_kyc_verified && self.is_active && self.revoked_at.is_none()
    }
}

// ─── Wallet ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    pub wallet_id: String,
    pub keypair: KeyPair,
    /// PECU coin balance (18 decimal precision, stored as u128 in smallest unit)
    pub pecu_balance: u128,
    /// Token balances: contract_address -> amount
    pub token_balances: std::collections::HashMap<String, u128>,
    pub created_at: i64,
    pub last_updated: i64,
    /// One validator node per wallet (whitepaper policy)
    pub validator_node_id: Option<String>,
    pub gak_sessions: Vec<GeneralAccessKey>,
    pub dak: Option<DevelopmentAccessKey>,
    /// Cold storage: key -> amount locked offline
    pub cold_storage: std::collections::HashMap<String, u128>,
}

impl Wallet {
    /// PECU coins are divisible to 15 decimal places (original whitepaper)
    pub const DECIMAL_PLACES: u32 = 15;
    /// We store in units of 10^15 for precision
    pub const UNIT_FACTOR: u128 = 1_000_000_000_000_000u128;

    pub fn new() -> Self {
        let keypair = KeyPair::generate();
        let now = Utc::now().timestamp();
        Wallet {
            wallet_id: crate::crypto::generate_uuid(),
            keypair,
            pecu_balance: 0,
            token_balances: std::collections::HashMap::new(),
            created_at: now,
            last_updated: now,
            validator_node_id: None,
            gak_sessions: Vec::new(),
            dak: None,
            cold_storage: std::collections::HashMap::new(),
        }
    }

    pub fn evm_address(&self) -> &str {
        &self.keypair.evm_address
    }

    pub fn pecu_address(&self) -> &str {
        &self.keypair.pecu_address
    }

    pub fn pecu_balance_display(&self) -> f64 {
        self.pecu_balance as f64 / Self::UNIT_FACTOR as f64
    }

    pub fn credit(&mut self, amount: u128) {
        self.pecu_balance = self.pecu_balance.saturating_add(amount);
        self.last_updated = Utc::now().timestamp();
    }

    pub fn debit(&mut self, amount: u128) -> bool {
        if self.pecu_balance >= amount {
            self.pecu_balance -= amount;
            self.last_updated = Utc::now().timestamp();
            true
        } else {
            false
        }
    }

    /// Move assets to cold storage (CSS feature from whitepaper)
    pub fn move_to_cold_storage(&mut self, amount: u128) -> Option<String> {
        if !self.debit(amount) { return None; }
        let storage_key = format!("CSS_{}", crypto::generate_public_key());
        self.cold_storage.insert(storage_key.clone(), amount);
        Some(storage_key)
    }

    /// Redeem from cold storage using the unique key
    pub fn redeem_from_cold_storage(&mut self, storage_key: &str) -> bool {
        if let Some(amount) = self.cold_storage.remove(storage_key) {
            self.credit(amount);
            true
        } else {
            false
        }
    }

    /// Create a GAK session for an app
    pub fn connect_to_app(&mut self, app_id: &str, ttl: Option<i64>) -> GeneralAccessKey {
        let gak = GeneralAccessKey::new(self.evm_address(), app_id, ttl);
        let result = gak.clone();
        self.gak_sessions.push(gak);
        result
    }

    pub fn disconnect_from_app(&mut self, app_id: &str) {
        for gak in &mut self.gak_sessions {
            if gak.app_id == app_id {
                gak.disconnect();
            }
        }
    }
}

impl Default for Wallet {
    fn default() -> Self {
        Self::new()
    }
}
