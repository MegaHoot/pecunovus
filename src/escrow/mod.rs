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

// escrow/mod.rs
// Pecu Novus MVault Escrow System
//
// Whitepaper: "Users can send assets with a dated release option.
// The sender retains full control, can cancel at any time if recipient
// doesn't fulfill contractual obligations."
// "Transfer Cards: unique key that can be scanned to redeem stored tokens."

use crate::crypto;
use serde::{Deserialize, Serialize};
use chrono::Utc;


// ─── Escrow Status ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EscrowStatus {
    /// Coins locked, awaiting release date
    Locked,
    /// Release date reached, coins automatically transferred
    Released,
    /// Sender canceled before release
    Canceled,
    /// Under dispute (upcoming feature per whitepaper)
    Disputed,
    /// Expired (for Transfer Cards with time limit)
    Expired,
}

// ─── Escrow Contract ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscrowContract {
    pub escrow_id: String,
    /// Unique key: "random length 64-128, combination of strings, integers, timestamp"
    pub escrow_key: String,
    pub sender: String,
    pub receiver: String,
    /// Amount of PECU locked (in smallest unit)
    pub amount: u128,
    /// Optional token contract address (for PNP16/ERC-20 tokens)
    pub token_contract: Option<String>,
    pub token_amount: Option<u128>,

    pub status: EscrowStatus,
    pub release_date: i64,
    pub created_at: i64,
    pub released_at: Option<i64>,
    pub canceled_at: Option<i64>,

    /// Public note (on-chain visible)
    pub note: Option<String>,
    /// Private note (only sender/receiver)
    pub private_note: Option<String>,
    /// Contract terms / agreement highlights
    pub agreement_details: Option<String>,

    /// Required actions before release
    pub required_actions: Vec<String>,
    /// Actions completed by receiver
    pub completed_actions: Vec<String>,

    /// For Transfer Cards: expiry (tokens revert to issuer on expiry)
    pub is_transfer_card: bool,
    pub transfer_card_expiry: Option<i64>,
    /// The smart-contract hash registered on-chain
    pub on_chain_hash: String,
}

impl EscrowContract {
    pub fn new(
        sender: &str,
        receiver: &str,
        amount: u128,
        release_date: i64,
        note: Option<String>,
        private_note: Option<String>,
        agreement_details: Option<String>,
        required_actions: Vec<String>,
    ) -> Self {
        let now = Utc::now().timestamp();
        let escrow_id = crate::crypto::generate_uuid();

        // Escrow key: whitepaper spec - random, SHA-512 based
        let raw_key = format!("{sender}{receiver}{amount}{release_date}{now}");
        let escrow_key = crypto::sha512(raw_key.as_bytes());

        // On-chain hash
        let chain_data = format!("{escrow_id}{sender}{receiver}{amount}{release_date}");
        let on_chain_hash = crypto::sha256(chain_data.as_bytes());

        EscrowContract {
            escrow_id,
            escrow_key,
            sender: sender.to_string(),
            receiver: receiver.to_string(),
            amount,
            token_contract: None,
            token_amount: None,
            status: EscrowStatus::Locked,
            release_date,
            created_at: now,
            released_at: None,
            canceled_at: None,
            note,
            private_note,
            agreement_details,
            required_actions,
            completed_actions: Vec::new(),
            is_transfer_card: false,
            transfer_card_expiry: None,
            on_chain_hash,
        }
    }

    /// Create with token (PNP16/ERC-20) instead of raw PECU
    pub fn new_token_escrow(
        sender: &str,
        receiver: &str,
        token_contract: &str,
        token_amount: u128,
        release_date: i64,
        note: Option<String>,
    ) -> Self {
        let mut escrow = Self::new(sender, receiver, 0, release_date, note, None, None, vec![]);
        escrow.token_contract = Some(token_contract.to_string());
        escrow.token_amount = Some(token_amount);
        escrow
    }

    /// Try to release: checks if release_date has passed and actions completed
    pub fn try_release(&mut self) -> bool {
        let now = Utc::now().timestamp();
        if self.status != EscrowStatus::Locked { return false; }

        // Check Transfer Card expiry first
        if self.is_transfer_card {
            if let Some(exp) = self.transfer_card_expiry {
                if now >= exp {
                    self.status = EscrowStatus::Expired;
                    return false; // tokens revert to issuer
                }
            }
        }

        // Check all required actions completed
        let all_done = self.required_actions.iter()
            .all(|a| self.completed_actions.contains(a));

        if now >= self.release_date && all_done {
            self.status = EscrowStatus::Released;
            self.released_at = Some(now);
            true
        } else {
            false
        }
    }

    /// Early release by sender
    pub fn release_early(&mut self) -> bool {
        if self.status != EscrowStatus::Locked { return false; }
        self.status = EscrowStatus::Released;
        self.released_at = Some(Utc::now().timestamp());
        true
    }

    /// Cancel by sender (before fulfillment)
    pub fn cancel(&mut self) -> bool {
        if self.status != EscrowStatus::Locked { return false; }
        self.status = EscrowStatus::Canceled;
        self.canceled_at = Some(Utc::now().timestamp());
        true
    }

    /// Mark a required action as completed by receiver
    pub fn complete_action(&mut self, action: &str) {
        if self.required_actions.contains(&action.to_string())
            && !self.completed_actions.contains(&action.to_string())
        {
            self.completed_actions.push(action.to_string());
        }
    }

    /// Dispute (upcoming feature per whitepaper)
    pub fn raise_dispute(&mut self) {
        if self.status == EscrowStatus::Locked {
            self.status = EscrowStatus::Disputed;
        }
    }

    pub fn is_expired(&self) -> bool {
        self.status == EscrowStatus::Expired
    }
}

// ─── Transfer Card ────────────────────────────────────────────────────────────
// Whitepaper: "Each Transfer Card embeds a unique key that can be scanned
// to redeem stored tokens in a Pecu Wallet. Cards can be digital or physical."

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferCard {
    pub card_id: String,
    /// Unique redemption key (like a gift card code)
    pub redemption_key: String,
    pub issuer: String,
    pub amount: u128,
    pub token_contract: Option<String>,
    pub is_redeemed: bool,
    pub redeemed_by: Option<String>,
    pub redeemed_at: Option<i64>,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub use_case: TransferCardUseCase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransferCardUseCase {
    EventGiveaway,
    GiftingDigitalAssets,
    TokenLaunch,
    MarketingCampaign,
    Custom(String),
}

impl TransferCard {
    pub fn new(
        issuer: &str,
        amount: u128,
        token_contract: Option<String>,
        expires_at: Option<i64>,
        use_case: TransferCardUseCase,
    ) -> Self {
        let now = Utc::now().timestamp();
        let raw = format!("{issuer}{amount}{now}{}", crate::crypto::generate_uuid());
        let redemption_key = crypto::sha512(raw.as_bytes());

        TransferCard {
            card_id: crate::crypto::generate_uuid(),
            redemption_key,
            issuer: issuer.to_string(),
            amount,
            token_contract,
            is_redeemed: false,
            redeemed_by: None,
            redeemed_at: None,
            created_at: now,
            expires_at,
            use_case,
        }
    }

    pub fn redeem(&mut self, redeemer: &str) -> Result<u128, String> {
        let now = Utc::now().timestamp();

        if self.is_redeemed {
            return Err("TransferCard already redeemed".to_string());
        }
        if let Some(exp) = self.expires_at {
            if now >= exp {
                return Err("TransferCard has expired — tokens reverted to issuer".to_string());
            }
        }

        self.is_redeemed = true;
        self.redeemed_by = Some(redeemer.to_string());
        self.redeemed_at = Some(now);
        Ok(self.amount)
    }

    pub fn is_valid(&self) -> bool {
        if self.is_redeemed { return false; }
        if let Some(exp) = self.expires_at {
            return Utc::now().timestamp() < exp;
        }
        true
    }
}

// ─── MVault ───────────────────────────────────────────────────────────────────
// Whitepaper: "MVault serves as both a secure vault for digital assets and
// the engine behind simplified smart contract deployment."

#[derive(Debug, Default)]
pub struct MVault {
    pub escrows: Vec<EscrowContract>,
    pub transfer_cards: Vec<TransferCard>,
}

impl MVault {
    pub fn new() -> Self { Self::default() }

    pub fn create_escrow(
        &mut self,
        sender: &str,
        receiver: &str,
        amount: u128,
        release_date: i64,
        note: Option<String>,
        private_note: Option<String>,
        agreement: Option<String>,
        required_actions: Vec<String>,
    ) -> EscrowContract {
        let contract = EscrowContract::new(
            sender, receiver, amount, release_date,
            note, private_note, agreement, required_actions,
        );
        let result = contract.clone();
        self.escrows.push(contract);
        result
    }

    pub fn get_escrow_mut(&mut self, escrow_id: &str) -> Option<&mut EscrowContract> {
        self.escrows.iter_mut().find(|e| e.escrow_id == escrow_id)
    }

    pub fn get_escrow(&self, escrow_id: &str) -> Option<&EscrowContract> {
        self.escrows.iter().find(|e| e.escrow_id == escrow_id)
    }

    pub fn pending_escrows_for(&self, address: &str) -> Vec<&EscrowContract> {
        self.escrows.iter()
            .filter(|e| e.status == EscrowStatus::Locked
                && (e.sender == address || e.receiver == address))
            .collect()
    }

    pub fn create_transfer_card(
        &mut self,
        issuer: &str,
        amount: u128,
        token_contract: Option<String>,
        expires_at: Option<i64>,
        use_case: TransferCardUseCase,
    ) -> TransferCard {
        let card = TransferCard::new(issuer, amount, token_contract, expires_at, use_case);
        let result = card.clone();
        self.transfer_cards.push(card);
        result
    }

    pub fn redeem_transfer_card(
        &mut self,
        redemption_key: &str,
        redeemer: &str,
    ) -> Result<u128, String> {
        let card = self.transfer_cards.iter_mut()
            .find(|c| c.redemption_key == redemption_key)
            .ok_or("Transfer card not found")?;
        card.redeem(redeemer)
    }

    /// Process all escrows that are ready for automatic release
    pub fn process_auto_releases(&mut self) -> Vec<String> {
        let mut released_ids = Vec::new();
        for escrow in &mut self.escrows {
            if escrow.status == EscrowStatus::Locked && escrow.try_release() {
                released_ids.push(escrow.escrow_id.clone());
            }
        }
        released_ids
    }
}
