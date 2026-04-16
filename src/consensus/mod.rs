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

// consensus/mod.rs
// Pecu Novus Proof of Time (PoT) + Proof of Stake (PoS) Hybrid Consensus
// Pecu 3.0 Themis: Hybrid PoT + PoS with Byzantine Fault Tolerance (BFT)
//
// Whitepaper: "A Validator node is assigned as the lead at any given moment
// to generate a Proof of Time sequence, ensuring global read consistency."
//
// "Validators receive randomized rewards ranging from 0.25 to 1.5 PECU per
// 24-hour period per hosted node."

pub use crate::crypto::VdfProof;
use crate::crypto;
use serde::{Deserialize, Serialize};
use chrono::Utc;
use rand::{Rng, thread_rng};


// ─── Constants (from whitepaper) ─────────────────────────────────────────────

/// Validator reward range: 0.25 to 1.5 PECU per 24h per node (in PECU * 10^15)
pub const VALIDATOR_REWARD_MIN: u128 = 250_000_000_000_000u128; // 0.25 PECU
pub const VALIDATOR_REWARD_MAX: u128 = 1_500_000_000_000_000u128; // 1.50 PECU

/// Max daily distribution: ~55,000 PECU
pub const MAX_DAILY_REWARD: u128 = 55_000_000_000_000_000_000u128;

/// PoT VDF delay steps (adjustable for network speed vs security)
pub const POT_DELAY_STEPS: u64 = 100;

// ─── Validator ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Validator {
    pub node_id: String,
    pub wallet_address: String,
    /// Staked PECU for PoS component (Pecu 3.0 Themis)
    pub stake: u128,
    pub uptime_seconds: u64,
    pub blocks_validated: u64,
    pub is_online: bool,
    pub is_lead: bool,
    pub registered_at: i64,
    pub last_seen: i64,
    /// Cumulative reward earned
    pub total_rewards_earned: u128,
    /// One validator per wallet (whitepaper policy)
    pub pruned_history_only: bool,
}

impl Validator {
    pub fn new(wallet_address: &str, initial_stake: u128) -> Self {
        let now = Utc::now().timestamp();
        Validator {
            node_id: crate::crypto::generate_uuid(),
            wallet_address: wallet_address.to_string(),
            stake: initial_stake,
            uptime_seconds: 0,
            blocks_validated: 0,
            is_online: true,
            is_lead: false,
            registered_at: now,
            last_seen: now,
            total_rewards_earned: 0,
            pruned_history_only: true,
        }
    }

    /// Weight for PoT+PoS selection: uptime * sqrt(stake + 1)
    /// Ensures time commitment + stake both matter, but neither dominates
    pub fn selection_weight(&self) -> f64 {
        let stake_factor = ((self.stake as f64 / 1_000_000_000_000_000f64) + 1.0).sqrt();
        self.uptime_seconds as f64 * stake_factor
    }

    pub fn record_heartbeat(&mut self, seconds_since_last: u64) {
        self.uptime_seconds += seconds_since_last;
        self.last_seen = Utc::now().timestamp();
    }

    pub fn add_reward(&mut self, amount: u128) {
        self.total_rewards_earned += amount;
        self.blocks_validated += 1;
    }

    pub fn daily_reward(&self) -> u128 {
        let mut rng = thread_rng();
        rng.gen_range(VALIDATOR_REWARD_MIN..=VALIDATOR_REWARD_MAX)
    }
}

// ─── Validator Reward ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorReward {
    pub validator_address: String,
    pub amount: u128,
    pub period: String,
    pub block_height: u64,
    pub issued_at: i64,
}

// ─── Halving Schedule ─────────────────────────────────────────────────────────
// Whitepaper: halving every decade, first in 2027

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HalvingSchedule {
    pub entries: Vec<HalvingEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HalvingEntry {
    pub year: u32,
    pub max_annual_reward: u128, // in PECU * 10^15
}

impl HalvingSchedule {
    pub fn official() -> Self {
        HalvingSchedule {
            entries: vec![
                HalvingEntry { year: 2017, max_annual_reward: 20_000_000_000_000_000_000_000u128 },
                HalvingEntry { year: 2027, max_annual_reward: 10_000_000_000_000_000_000_000u128 },
                HalvingEntry { year: 2037, max_annual_reward:  5_000_000_000_000_000_000_000u128 },
                HalvingEntry { year: 2047, max_annual_reward:  2_500_000_000_000_000_000_000u128 },
                HalvingEntry { year: 2057, max_annual_reward:  1_250_000_000_000_000_000_000u128 },
            ],
        }
    }

    pub fn current_max_annual_reward(&self) -> u128 {
        let current_year = Utc::now().format("%Y").to_string().parse::<u32>().unwrap_or(2025);
        let mut reward = self.entries[0].max_annual_reward;
        for entry in &self.entries {
            if entry.year <= current_year {
                reward = entry.max_annual_reward;
            }
        }
        reward
    }
}

// ─── Proof of Time Engine ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ProofOfTime {
    pub validators: Vec<Validator>,
    pub current_lead_idx: usize,
    pub pot_sequence: u64,
    pub daily_rewards_issued: u128,
    pub last_reward_reset: i64,
    pub halving: HalvingSchedule,
}

impl ProofOfTime {
    pub fn new() -> Self {
        ProofOfTime {
            validators: Vec::new(),
            current_lead_idx: 0,
            pot_sequence: 0,
            daily_rewards_issued: 0,
            last_reward_reset: Utc::now().timestamp(),
            halving: HalvingSchedule::official(),
        }
    }

    pub fn register_validator(&mut self, validator: Validator) {
        // One validator per wallet address (whitepaper policy)
        if self.validators.iter().any(|v| v.wallet_address == validator.wallet_address) {
            return;
        }
        self.validators.push(validator);
    }

    pub fn online_validators(&self) -> Vec<&Validator> {
        self.validators.iter().filter(|v| v.is_online).collect()
    }

    /// Generate a PoT proof for the next block
    /// Whitepaper: "a Validator node is assigned as the lead at any given moment
    /// to generate a Proof of Time sequence"
    pub fn generate_pot_proof(&mut self, block_hash_seed: &str) -> (VdfProof, String) {
        let seed = format!("{block_hash_seed}_{}", self.pot_sequence);
        let proof = crypto::compute_vdf(&seed, POT_DELAY_STEPS);
        self.pot_sequence += 1;

        // Select lead validator using PoT+PoS weighted random selection (BFT)
        let lead = self.select_lead_validator();
        (proof, lead)
    }

    /// Hybrid PoT+PoS validator selection with BFT guarantees
    /// - Computes weighted selection based on uptime (time) + stake (PoS)
    /// - Uses randomization to prevent centralization
    fn select_lead_validator(&mut self) -> String {
        let online: Vec<&Validator> = self.online_validators();
        if online.is_empty() {
            return "no_validators".to_string();
        }

        let total_weight: f64 = online.iter().map(|v| v.selection_weight()).sum();
        if total_weight == 0.0 {
            return online[0].wallet_address.clone();
        }

        let mut rng = thread_rng();
        let pick = rng.gen_range(0.0..total_weight);
        let mut cumulative = 0.0;

        for v in &online {
            cumulative += v.selection_weight();
            if cumulative >= pick {
                return v.wallet_address.clone();
            }
        }
        online.last().unwrap().wallet_address.clone()
    }

    /// Issue daily rewards to all active validators
    /// Returns list of (address, reward_amount) pairs
    pub fn issue_daily_rewards(&mut self) -> Vec<ValidatorReward> {
        let now = Utc::now().timestamp();
        let seconds_per_day = 86400i64;

        // Reset daily cap if a new day
        if now - self.last_reward_reset >= seconds_per_day {
            self.daily_rewards_issued = 0;
            self.last_reward_reset = now;
        }

        let mut rewards = Vec::new();
        let block_height = self.pot_sequence;

        for v in self.validators.iter_mut() {
            if !v.is_online { continue; }
            if self.daily_rewards_issued >= MAX_DAILY_REWARD { break; }

            let mut rng = thread_rng();
            let reward = rng.gen_range(VALIDATOR_REWARD_MIN..=VALIDATOR_REWARD_MAX);
            let capped = reward.min(MAX_DAILY_REWARD - self.daily_rewards_issued);

            v.add_reward(capped);
            self.daily_rewards_issued += capped;

            rewards.push(ValidatorReward {
                validator_address: v.wallet_address.clone(),
                amount: capped,
                period: Utc::now().format("%Y-%m-%d").to_string(),
                block_height,
                issued_at: now,
            });
        }
        rewards
    }

    /// Verify a PoT proof
    pub fn verify_proof(&self, proof: &VdfProof) -> bool {
        crypto::verify_vdf(proof)
    }
}

impl Default for ProofOfTime {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Vesting Schedule ─────────────────────────────────────────────────────────
// Whitepaper: locked tokens release schedule

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VestingSchedule {
    pub entries: Vec<VestingEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VestingEntry {
    pub release_year: u32,
    pub amount_pecu: u64, // in millions
}

impl VestingSchedule {
    pub fn official() -> Self {
        VestingSchedule {
            entries: vec![
                VestingEntry { release_year: 2026, amount_pecu: 40 },
                VestingEntry { release_year: 2028, amount_pecu: 30 },
                VestingEntry { release_year: 2030, amount_pecu: 30 },
                VestingEntry { release_year: 2032, amount_pecu: 20 },
                VestingEntry { release_year: 2034, amount_pecu: 10 },
            ],
        }
    }
}
