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

// tokens/mod.rs
// Pecu Novus Token Standards:
//   - PNP16: Native Pecu Novus token standard
//   - ERC-20: Full EVM-compatible fungible token standard
//   - ERC-1400: Security token standard (Pecu 3.0 Themis)
//
// Whitepaper: "PNP16 tokens can represent financial assets, gaming assets,
// physical commodities, fractional real estate ownership."
// "Tokens exist as a single instance on the Pecu Novus mainnet."

use crate::crypto;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Token Standard Enum ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TokenStandard {
    PNP16,
    ERC20,
    ERC1400,
    NFT, // Non-Fungible Token
}

// ─── Token Asset Class (PNP16) ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AssetClass {
    FinancialAsset,       // Company stake/ownership
    GamingAsset,          // In-game rewards, earned points, character skills
    PhysicalCommodity,    // Gold, silver, oil, agricultural products
    FractionalRealEstate, // Tokenized property ownership
    IntellectualProperty, // Music, film, software licensing
    Stablecoin,           // Pegged to fiat or commodity
    SecurityToken,        // ERC-1400 regulated security
    Utility,              // General utility token
}

// ─── PNP16 Token ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PNP16Token {
    pub contract_address: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: u128,
    pub max_supply: Option<u128>,
    pub asset_class: AssetClass,
    pub creator: String,
    pub created_at: i64,
    /// All transactions recorded on mainnet (Pecuscan visible)
    pub is_public: bool,
    /// Balances: address -> amount
    pub balances: HashMap<String, u128>,
    /// Allowances: (owner, spender) -> amount  [ERC-20 compat]
    pub allowances: HashMap<(String, String), u128>,
    /// Smart contract parameters set at creation
    pub params: HashMap<String, String>,
    /// Subset mainnet for this token's own transaction records
    pub subset_ledger: Vec<TokenTransaction>,
    /// DAK of the developer who deployed this token
    pub deployer_dak: String,
}

impl PNP16Token {
    pub fn new(
        name: &str,
        symbol: &str,
        decimals: u8,
        initial_supply: u128,
        max_supply: Option<u128>,
        asset_class: AssetClass,
        creator: &str,
        deployer_dak: &str,
    ) -> Self {
        let contract_address = format!(
            "0x{}",
            &crypto::keccak256(
                format!(
                    "{name}{symbol}{creator}{}",
                    Utc::now().timestamp_nanos_opt().unwrap_or(0)
                )
                .as_bytes()
            )[..40]
        );

        let mut balances = HashMap::new();
        if initial_supply > 0 {
            balances.insert(creator.to_string(), initial_supply);
        }

        PNP16Token {
            contract_address,
            name: name.to_string(),
            symbol: symbol.to_string(),
            decimals,
            total_supply: initial_supply,
            max_supply,
            asset_class,
            creator: creator.to_string(),
            created_at: Utc::now().timestamp(),
            is_public: true,
            balances,
            allowances: HashMap::new(),
            params: HashMap::new(),
            subset_ledger: Vec::new(),
            deployer_dak: deployer_dak.to_string(),
        }
    }

    // ── ERC-20 Interface ─────────────────────────────────────────────────────

    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn symbol(&self) -> &str {
        &self.symbol
    }
    pub fn decimals(&self) -> u8 {
        self.decimals
    }
    pub fn total_supply(&self) -> u128 {
        self.total_supply
    }

    pub fn balance_of(&self, address: &str) -> u128 {
        *self.balances.get(address).unwrap_or(&0)
    }

    /// ERC-20: transfer(to, amount) — called by token holder
    pub fn transfer(&mut self, from: &str, to: &str, amount: u128) -> Result<bool, String> {
        let from_balance = self.balance_of(from);
        if from_balance < amount {
            return Err(format!(
                "ERC20: transfer amount exceeds balance ({} < {})",
                from_balance, amount
            ));
        }

        *self.balances.entry(from.to_string()).or_insert(0) -= amount;
        *self.balances.entry(to.to_string()).or_insert(0) += amount;

        self.record_tx(TokenTransaction::transfer(
            from,
            to,
            amount,
            &self.contract_address,
        ));
        Ok(true)
    }

    /// ERC-20: approve(spender, amount)
    pub fn approve(&mut self, owner: &str, spender: &str, amount: u128) -> Result<bool, String> {
        self.allowances
            .insert((owner.to_string(), spender.to_string()), amount);
        Ok(true)
    }

    /// ERC-20: allowance(owner, spender)
    pub fn allowance(&self, owner: &str, spender: &str) -> u128 {
        *self
            .allowances
            .get(&(owner.to_string(), spender.to_string()))
            .unwrap_or(&0)
    }

    /// ERC-20: transferFrom(from, to, amount) — called by approved spender
    pub fn transfer_from(
        &mut self,
        spender: &str,
        from: &str,
        to: &str,
        amount: u128,
    ) -> Result<bool, String> {
        let allowed = self.allowance(from, spender);
        if allowed < amount {
            return Err(format!(
                "ERC20: insufficient allowance ({} < {})",
                allowed, amount
            ));
        }
        let from_balance = self.balance_of(from);
        if from_balance < amount {
            return Err(format!("ERC20: transfer amount exceeds balance"));
        }

        // Reduce allowance
        *self
            .allowances
            .entry((from.to_string(), spender.to_string()))
            .or_insert(0) -= amount;
        // Move tokens
        *self.balances.entry(from.to_string()).or_insert(0) -= amount;
        *self.balances.entry(to.to_string()).or_insert(0) += amount;

        self.record_tx(TokenTransaction::transfer(
            from,
            to,
            amount,
            &self.contract_address,
        ));
        Ok(true)
    }

    /// Mint new tokens (up to max_supply)
    pub fn mint(&mut self, to: &str, amount: u128) -> Result<bool, String> {
        if let Some(max) = self.max_supply {
            if self.total_supply + amount > max {
                return Err(format!(
                    "PNP16: mint would exceed max supply ({} + {} > {})",
                    self.total_supply, amount, max
                ));
            }
        }
        self.total_supply += amount;
        *self.balances.entry(to.to_string()).or_insert(0) += amount;
        self.record_tx(TokenTransaction::mint(to, amount, &self.contract_address));
        Ok(true)
    }

    /// Burn tokens (reduce supply)
    pub fn burn(&mut self, from: &str, amount: u128) -> Result<bool, String> {
        let bal = self.balance_of(from);
        if bal < amount {
            return Err("PNP16: burn amount exceeds balance".to_string());
        }
        *self.balances.entry(from.to_string()).or_insert(0) -= amount;
        self.total_supply -= amount;
        self.record_tx(TokenTransaction::burn(from, amount, &self.contract_address));
        Ok(true)
    }

    fn record_tx(&mut self, tx: TokenTransaction) {
        self.subset_ledger.push(tx);
    }
}

// ─── ERC-20 Token (alias / wrapper for full EVM compat) ──────────────────────

pub type ERC20Token = PNP16Token;

// ─── ERC-1400 Security Token (Pecu 3.0 Themis) ───────────────────────────────
// Adds partitions, operators, issuance controls for regulated securities

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ERC1400Token {
    pub base: PNP16Token,
    /// Partitions: partition_name -> (address -> amount)
    pub partitions: HashMap<String, HashMap<String, u128>>,
    /// Operator approvals: (operator, token_holder) -> can_operate
    pub operators: HashMap<(String, String), bool>,
    /// KYC-verified holders
    pub verified_holders: Vec<String>,
    /// Token is issuable
    pub is_issuable: bool,
    pub controllers: Vec<String>,
}

impl ERC1400Token {
    pub fn new(base: PNP16Token, controllers: Vec<String>) -> Self {
        ERC1400Token {
            base,
            partitions: HashMap::new(),
            operators: HashMap::new(),
            verified_holders: Vec::new(),
            is_issuable: true,
            controllers,
        }
    }

    pub fn issue_by_partition(
        &mut self,
        partition: &str,
        to: &str,
        amount: u128,
    ) -> Result<bool, String> {
        if !self.is_issuable {
            return Err("ERC1400: token is not issuable".to_string());
        }
        let part = self.partitions.entry(partition.to_string()).or_default();
        *part.entry(to.to_string()).or_insert(0) += amount;
        self.base.total_supply += amount;
        *self.base.balances.entry(to.to_string()).or_insert(0) += amount;
        Ok(true)
    }

    pub fn balance_of_by_partition(&self, partition: &str, address: &str) -> u128 {
        self.partitions
            .get(partition)
            .and_then(|p| p.get(address))
            .copied()
            .unwrap_or(0)
    }

    pub fn add_verified_holder(&mut self, address: &str) {
        if !self.verified_holders.contains(&address.to_string()) {
            self.verified_holders.push(address.to_string());
        }
    }

    pub fn is_verified_holder(&self, address: &str) -> bool {
        self.verified_holders.contains(&address.to_string())
    }

    pub fn authorize_operator(&mut self, operator: &str, holder: &str) {
        self.operators
            .insert((operator.to_string(), holder.to_string()), true);
    }

    pub fn is_operator(&self, operator: &str, holder: &str) -> bool {
        *self
            .operators
            .get(&(operator.to_string(), holder.to_string()))
            .unwrap_or(&false)
    }
}

// ─── Token Transaction (subset ledger) ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TokenTxType {
    Transfer,
    Mint,
    Burn,
    Approve,
    TransferFrom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenTransaction {
    pub tx_id: String,
    pub tx_type: TokenTxType,
    pub from: Option<String>,
    pub to: Option<String>,
    pub amount: u128,
    pub contract_address: String,
    pub timestamp: i64,
    pub on_mainnet: bool,
}

impl TokenTransaction {
    pub fn transfer(from: &str, to: &str, amount: u128, contract: &str) -> Self {
        TokenTransaction {
            tx_id: crate::crypto::generate_uuid(),
            tx_type: TokenTxType::Transfer,
            from: Some(from.to_string()),
            to: Some(to.to_string()),
            amount,
            contract_address: contract.to_string(),
            timestamp: Utc::now().timestamp(),
            on_mainnet: true,
        }
    }

    pub fn mint(to: &str, amount: u128, contract: &str) -> Self {
        TokenTransaction {
            tx_id: crate::crypto::generate_uuid(),
            tx_type: TokenTxType::Mint,
            from: None,
            to: Some(to.to_string()),
            amount,
            contract_address: contract.to_string(),
            timestamp: Utc::now().timestamp(),
            on_mainnet: true,
        }
    }

    pub fn burn(from: &str, amount: u128, contract: &str) -> Self {
        TokenTransaction {
            tx_id: crate::crypto::generate_uuid(),
            tx_type: TokenTxType::Burn,
            from: Some(from.to_string()),
            to: None,
            amount,
            contract_address: contract.to_string(),
            timestamp: Utc::now().timestamp(),
            on_mainnet: true,
        }
    }
}

// ─── Token Registry ───────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct TokenRegistry {
    pub tokens: HashMap<String, PNP16Token>,
    pub security_tokens: HashMap<String, ERC1400Token>,
}

impl TokenRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn deploy_pnp16(&mut self, token: PNP16Token) -> String {
        let addr = token.contract_address.clone();
        self.tokens.insert(addr.clone(), token);
        addr
    }

    pub fn deploy_erc1400(&mut self, token: ERC1400Token) -> String {
        let addr = token.base.contract_address.clone();
        self.security_tokens.insert(addr.clone(), token);
        addr
    }

    pub fn get_token(&self, address: &str) -> Option<&PNP16Token> {
        self.tokens.get(address)
    }

    pub fn get_token_mut(&mut self, address: &str) -> Option<&mut PNP16Token> {
        self.tokens.get_mut(address)
    }

    pub fn get_security_token(&self, address: &str) -> Option<&ERC1400Token> {
        self.security_tokens.get(address)
    }

    pub fn get_security_token_mut(&mut self, address: &str) -> Option<&mut ERC1400Token> {
        self.security_tokens.get_mut(address)
    }

    pub fn list_tokens(&self) -> Vec<TokenSummary> {
        let mut result: Vec<TokenSummary> = self
            .tokens
            .values()
            .map(|t| TokenSummary {
                contract_address: t.contract_address.clone(),
                name: t.name.clone(),
                symbol: t.symbol.clone(),
                standard: TokenStandard::PNP16,
                total_supply: t.total_supply,
                decimals: t.decimals,
            })
            .collect();

        for t in self.security_tokens.values() {
            result.push(TokenSummary {
                contract_address: t.base.contract_address.clone(),
                name: t.base.name.clone(),
                symbol: t.base.symbol.clone(),
                standard: TokenStandard::ERC1400,
                total_supply: t.base.total_supply,
                decimals: t.base.decimals,
            });
        }
        result
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSummary {
    pub contract_address: String,
    pub name: String,
    pub symbol: String,
    pub standard: TokenStandard,
    pub total_supply: u128,
    pub decimals: u8,
}
