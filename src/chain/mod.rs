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

// chain/mod.rs
// Pecu Novus blockchain core: Block, Transaction, Blockchain

use crate::consensus::VdfProof;
use crate::crypto;
use chrono::Utc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

// ─── Transaction Types ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionType {
    /// Standard PECU coin transfer
    Transfer,
    /// Escrow-locked transfer with dated release
    Escrow,
    /// Token minting (PNP16 / ERC-20)
    TokenMint,
    /// Token transfer
    TokenTransfer,
    /// Smart contract deployment
    ContractDeploy,
    /// Smart contract call
    ContractCall,
    /// NFT creation/transfer
    NFT,
    /// Validator reward
    ValidatorReward,
    /// Burn (gas fee burn, 50% of fees)
    Burn,
    /// ERC-20 approve
    ERC20Approve,
    /// ERC-20 transferFrom
    ERC20TransferFrom,
}

// ─── Transaction ──────────────────────────────────────────────────────────────
// Whitepaper block address fields: sender, receiver, amount, timestamp, escrow, note

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// SHA-512 based transaction hash
    pub tx_hash: String,
    pub tx_type: TransactionType,

    pub sender: String, // EVM address or Pecu address
    pub receiver: String,
    /// Amount in smallest PECU unit (10^-15)
    pub amount: u128,
    /// Gas fee: 0.0025% of amount (flat fee from whitepaper)
    pub gas_fee: u128,

    pub timestamp: i64,
    /// Optional public note (permanently on-chain)
    pub note: Option<String>,
    /// Private note (only sender/receiver can see — stored encrypted)
    pub private_note: Option<String>,

    pub is_escrow: bool,
    pub escrow_release_date: Option<i64>,

    /// For token transactions
    pub contract_address: Option<String>,
    pub token_id: Option<String>,
    pub call_data: Option<String>, // hex-encoded calldata for EVM

    /// Dual confirmation (whitepaper feature)
    pub sender_confirmed: bool,
    pub receiver_confirmed: bool,

    /// Signature: SHA-512 of (private_key + tx_data)
    pub signature: String,

    pub block_height: Option<u64>,
    pub nonce: u64,
}

impl Transaction {
    /// Gas fee: flat 0.0025% of amount (whitepaper spec)
    pub const GAS_FEE_RATE_BPS: u128 = 25; // 0.0025% = 25/1_000_000

    pub fn compute_gas_fee(amount: u128) -> u128 {
        amount.saturating_mul(Self::GAS_FEE_RATE_BPS) / 1_000_000
    }

    pub fn new(
        tx_type: TransactionType,
        sender: &str,
        receiver: &str,
        amount: u128,
        note: Option<String>,
        private_note: Option<String>,
        is_escrow: bool,
        escrow_release_date: Option<i64>,
        contract_address: Option<String>,
        nonce: u64,
    ) -> Self {
        let timestamp = Utc::now().timestamp();
        let gas_fee = Self::compute_gas_fee(amount);

        let tx_data = format!(
            "{sender}{receiver}{amount}{timestamp}{nonce}{}{}",
            note.as_deref().unwrap_or(""),
            is_escrow
        );
        let tx_hash = crypto::compute_block_address(
            sender,
            receiver,
            &amount.to_string(),
            timestamp,
            note.as_deref().unwrap_or(""),
            is_escrow,
        );

        Transaction {
            tx_hash,
            tx_type,
            sender: sender.to_string(),
            receiver: receiver.to_string(),
            amount,
            gas_fee,
            timestamp,
            note,
            private_note,
            is_escrow,
            escrow_release_date,
            contract_address,
            token_id: None,
            call_data: None,
            sender_confirmed: true,
            receiver_confirmed: false,
            signature: String::new(),
            block_height: None,
            nonce,
        }
    }

    pub fn sign(&mut self, private_key: &str) {
        let data = format!(
            "{}{}{}{}{}",
            self.tx_hash, self.sender, self.receiver, self.amount, self.timestamp
        );
        let combined = format!("{private_key}{data}");
        self.signature = crypto::sha512(combined.as_bytes());
    }

    pub fn confirm_by_receiver(&mut self) {
        self.receiver_confirmed = true;
    }

    pub fn is_fully_confirmed(&self) -> bool {
        self.sender_confirmed && self.receiver_confirmed
    }

    /// Compute burned amount (50% of gas fee)
    pub fn burned_amount(&self) -> u128 {
        self.gas_fee / 2
    }
}

// ─── Block Header ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    pub height: u64,
    pub previous_hash: String,
    pub merkle_root: String,
    pub timestamp: i64,
    pub validator: String,
    pub pot_proof: VdfProof,
    /// State hash (e.g. "02x63fde..." as shown in whitepaper diagrams)
    pub state_hash: String,
    pub version: u32,
    pub tx_count: u32,
}

impl BlockHeader {
    pub fn compute_hash(&self) -> String {
        let data = format!(
            "{}{}{}{}{}{}{}",
            self.height,
            self.previous_hash,
            self.merkle_root,
            self.timestamp,
            self.validator,
            self.pot_proof.output,
            self.state_hash,
        );
        crypto::sha256(data.as_bytes())
    }
}

// ─── Block ────────────────────────────────────────────────────────────────────
// Whitepaper: "Block #101 → Block #102, each linked via Previous block hash + Trans hash"

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub hash: String,
    pub transactions: Vec<Transaction>,
}

impl Block {
    pub fn new(
        height: u64,
        previous_hash: &str,
        transactions: Vec<Transaction>,
        validator: &str,
        pot_proof: VdfProof,
    ) -> Self {
        let timestamp = Utc::now().timestamp();
        let tx_hashes: Vec<String> = transactions.iter().map(|t| t.tx_hash.clone()).collect();
        let merkle_root = crypto::compute_merkle_root(&tx_hashes);

        // Compute state hash from all tx data + previous state
        let state_data = format!("{previous_hash}{merkle_root}{timestamp}{validator}");
        let state_hash = format!("0x{}", &crypto::sha256(state_data.as_bytes())[..8]);

        let header = BlockHeader {
            height,
            previous_hash: previous_hash.to_string(),
            merkle_root,
            timestamp,
            validator: validator.to_string(),
            pot_proof,
            state_hash,
            version: 2, // Pecu 2.0
            tx_count: transactions.len() as u32,
        };

        let hash = header.compute_hash();

        Block {
            header,
            hash,
            transactions,
        }
    }

    pub fn genesis() -> Self {
        let genesis_proof = VdfProof {
            input: "pecu_novus_genesis_2017".to_string(),
            output: crypto::sha256(b"pecu_novus_genesis_2017"),
            delay: 0,
            timestamp: 1484438400, // 2017-01-15 UTC
            sequence_count: 0,
        };

        let genesis_tx = Transaction {
            tx_hash: crypto::sha256(b"genesis"),
            tx_type: TransactionType::Transfer,
            sender: "0x0000000000000000000000000000000000000000".to_string(),
            receiver: "PecuNovusFoundation".to_string(),
            amount: 200_000_000_000_000_000_000_000u128, // 200M PECU initial supply
            gas_fee: 0,
            timestamp: 1484438400,
            note: Some("Pecu Novus Genesis Block - January 15, 2017".to_string()),
            private_note: None,
            is_escrow: false,
            escrow_release_date: None,
            contract_address: None,
            token_id: None,
            call_data: None,
            sender_confirmed: true,
            receiver_confirmed: true,
            signature: "genesis".to_string(),
            block_height: Some(0),
            nonce: 0,
        };

        Block::new(
            0,
            "0000000000000000000000000000000000000000000000000000000000000000",
            vec![genesis_tx],
            "PecuNovusFoundation",
            genesis_proof,
        )
    }

    pub fn total_fees(&self) -> u128 {
        self.transactions.iter().map(|t| t.gas_fee).sum()
    }

    pub fn total_burned(&self) -> u128 {
        self.transactions.iter().map(|t| t.burned_amount()).sum()
    }
}

// ─── Blockchain ───────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct Blockchain {
    pub chain: Arc<RwLock<Vec<Block>>>,
    /// Pending transactions (transaction queue → Validator queue → Smart Contract Validator)
    pub mempool: Arc<RwLock<Vec<Transaction>>>,
    /// Account balances: address -> PECU amount
    pub balances: Arc<RwLock<HashMap<String, u128>>>,
    /// Total burned PECU
    pub total_burned: Arc<RwLock<u128>>,
    /// Nonces: address -> nonce
    pub nonces: Arc<RwLock<HashMap<String, u64>>>,
    /// ERC-20 token allowances: (owner, spender, contract) -> amount
    pub allowances: Arc<RwLock<HashMap<(String, String, String), u128>>>,
}

impl Blockchain {
    /// Maximum supply: 1 billion PECU (capped, never exceeded)
    pub const MAX_SUPPLY: u128 = 1_000_000_000_000_000_000_000_000u128; // 1B * 10^15

    /// Daily Validator reward cap: 55,000 PECU
    pub const DAILY_VALIDATOR_REWARD_CAP: u128 = 55_000_000_000_000_000_000u128;

    /// Annual Validator reward cap: 20M PECU (first decade, until 2027)
    pub const ANNUAL_VALIDATOR_REWARD_CAP: u128 = 20_000_000_000_000_000_000_000u128;

    /// Gas fee burn: 50% of collected fees
    pub const BURN_RATIO: u128 = 50;

    pub fn new() -> Self {
        let genesis = Block::genesis();
        let mut balances = HashMap::new();

        // Initialize genesis balance
        for tx in &genesis.transactions {
            *balances.entry(tx.receiver.clone()).or_insert(0) += tx.amount;
        }

        Blockchain {
            chain: Arc::new(RwLock::new(vec![genesis])),
            mempool: Arc::new(RwLock::new(Vec::new())),
            balances: Arc::new(RwLock::new(balances)),
            total_burned: Arc::new(RwLock::new(0)),
            nonces: Arc::new(RwLock::new(HashMap::new())),
            allowances: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn latest_block(&self) -> Block {
        self.chain.read().last().unwrap().clone()
    }

    pub fn block_height(&self) -> u64 {
        self.chain.read().len() as u64 - 1
    }

    pub fn get_balance(&self, address: &str) -> u128 {
        *self.balances.read().get(address).unwrap_or(&0)
    }

    pub fn get_nonce(&self, address: &str) -> u64 {
        *self.nonces.read().get(address).unwrap_or(&0)
    }

    pub fn add_to_mempool(&self, tx: Transaction) -> Result<String, String> {
        // Validate transaction
        self.validate_transaction(&tx)?;
        let hash = tx.tx_hash.clone();
        self.mempool.write().push(tx);
        Ok(hash)
    }

    pub fn validate_transaction(&self, tx: &Transaction) -> Result<(), String> {
        let balances = self.balances.read();
        let sender_balance = balances.get(&tx.sender).copied().unwrap_or(0);
        let total_cost = tx.amount.saturating_add(tx.gas_fee);

        if tx.tx_type == TransactionType::Transfer || tx.tx_type == TransactionType::Escrow {
            if tx.sender != "0x0000000000000000000000000000000000000000"
                && sender_balance < total_cost
            {
                return Err(format!(
                    "Insufficient balance: {} < {}",
                    sender_balance, total_cost
                ));
            }
        }

        // Check nonce
        let expected_nonce = self.get_nonce(&tx.sender);
        if tx.nonce < expected_nonce {
            return Err(format!("Invalid nonce: {} < {}", tx.nonce, expected_nonce));
        }

        Ok(())
    }

    /// Commit a new block (called by Validator after PoT consensus)
    pub fn commit_block(&self, block: Block) -> Result<(), String> {
        // Apply all transactions
        {
            let mut balances = self.balances.write();
            let mut burned = self.total_burned.write();
            let mut nonces = self.nonces.write();

            for tx in &block.transactions {
                match tx.tx_type {
                    TransactionType::Transfer | TransactionType::Escrow => {
                        let sender_bal = balances.entry(tx.sender.clone()).or_insert(0);
                        if tx.sender != "0x0000000000000000000000000000000000000000" {
                            *sender_bal = sender_bal.saturating_sub(tx.amount + tx.gas_fee);
                        }
                        *balances.entry(tx.receiver.clone()).or_insert(0) += tx.amount;

                        // Burn 50% of gas fees
                        *burned += tx.burned_amount();

                        // Validator gets 50% of gas fee
                        *balances.entry(block.header.validator.clone()).or_insert(0) +=
                            tx.gas_fee - tx.burned_amount();
                    }
                    TransactionType::ValidatorReward => {
                        *balances.entry(tx.receiver.clone()).or_insert(0) += tx.amount;
                    }
                    TransactionType::Burn => {
                        let sender_bal = balances.entry(tx.sender.clone()).or_insert(0);
                        *sender_bal = sender_bal.saturating_sub(tx.amount);
                        *burned += tx.amount;
                    }
                    TransactionType::ERC20Approve => {
                        // allowance handled by token layer
                    }
                    _ => {
                        // Token and contract txs handled by token/escrow layers
                    }
                }

                // Increment nonce
                if tx.sender != "0x0000000000000000000000000000000000000000" {
                    let nonce = nonces.entry(tx.sender.clone()).or_insert(0);
                    *nonce += 1;
                }
            }
        }

        self.chain.write().push(block);
        Ok(())
    }

    /// Take up to `max_txs` pending transactions from mempool
    pub fn drain_mempool(&self, max_txs: usize) -> Vec<Transaction> {
        let mut pool = self.mempool.write();
        let drain_count = max_txs.min(pool.len());
        pool.drain(..drain_count).collect()
    }

    pub fn get_block_by_height(&self, height: u64) -> Option<Block> {
        self.chain.read().get(height as usize).cloned()
    }

    pub fn get_block_by_hash(&self, hash: &str) -> Option<Block> {
        self.chain.read().iter().find(|b| b.hash == hash).cloned()
    }

    pub fn get_transaction(&self, tx_hash: &str) -> Option<Transaction> {
        for block in self.chain.read().iter() {
            for tx in &block.transactions {
                if tx.tx_hash == tx_hash {
                    return Some(tx.clone());
                }
            }
        }
        None
    }

    /// ERC-20 style: approve spender for contract
    pub fn approve_erc20(&self, owner: &str, spender: &str, contract: &str, amount: u128) {
        let mut allowances = self.allowances.write();
        allowances.insert(
            (owner.to_string(), spender.to_string(), contract.to_string()),
            amount,
        );
    }

    pub fn get_allowance(&self, owner: &str, spender: &str, contract: &str) -> u128 {
        *self
            .allowances
            .read()
            .get(&(owner.to_string(), spender.to_string(), contract.to_string()))
            .unwrap_or(&0)
    }

    /// Chain statistics
    pub fn stats(&self) -> ChainStats {
        let chain = self.chain.read();
        let total_txs: usize = chain.iter().map(|b| b.transactions.len()).sum();
        ChainStats {
            block_height: chain.len() as u64 - 1,
            total_transactions: total_txs as u64,
            total_burned: *self.total_burned.read(),
            mempool_size: self.mempool.read().len() as u64,
            total_accounts: self.balances.read().len() as u64,
        }
    }
}

impl Default for Blockchain {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainStats {
    pub block_height: u64,
    pub total_transactions: u64,
    pub total_burned: u128,
    pub mempool_size: u64,
    pub total_accounts: u64,
}
