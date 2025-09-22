//! Proof of Stake registry & leader selection.
//!
//! This implementation supports:
//! - register(validator, stake)
//! - total_stake()
//! - select_leader(slot) -> best-effort (round-robin if no randomness)
//! - select_leader_with_seed(seed) -> deterministic weighted selection using seed bytes

use std::collections::HashMap;
use crate::consensus::types::ValidatorId;
use crate::consensus::types::hash_bytes;
use sha2::{Sha256, Digest};

#[derive(Debug, Clone)]
pub struct StakeInfo {
    pub validator: ValidatorId,
    pub stake: u64,
}

/// ProofOfStake holds validator stakes. In production this reads from on-chain stake accounts.
#[derive(Debug, Clone)]
pub struct ProofOfStake {
    stakes: HashMap<ValidatorId, u64>,
    ordered: Vec<StakeInfo>, // cache for deterministic iteration
    total: u64,
}

impl ProofOfStake {
    pub fn new() -> Self {
        Self {
            stakes: HashMap::new(),
            ordered: vec![],
            total: 0,
        }
    }

    pub fn register(&mut self, validator: ValidatorId, stake: u64) {
        self.total = self.total.saturating_sub(*self.stakes.get(&validator).unwrap_or(&0));
        self.stakes.insert(validator.clone(), stake);
        self.total = self.total.saturating_add(stake);
        // rebuild ordered
        self.ordered = self.stakes.iter().map(|(v, s)| StakeInfo { validator: v.clone(), stake: *s }).collect();
        // stable sort by validator id to keep determinism
        self.ordered.sort_by(|a, b| a.validator.cmp(&b.validator));
    }

    pub fn total_stake(&self) -> u64 {
        self.total
    }

    /// Select leader by slot with very simple deterministic rule: weighted by stake but using seed
    /// for deterministic selection: compute H(seed || slot) and map to stake range.
    pub fn select_leader_with_seed<T: AsRef<[u8]>>(&self, seed: T) -> Option<&ValidatorId> {
        if self.ordered.is_empty() || self.total == 0 {
            return None;
        }
        // hash the seed to a u128 number
        let mut hasher = Sha256::new();
        hasher.update(seed.as_ref());
        let digest = hasher.finalize();
        // take first 16 bytes as u128
        let mut arr = [0u8; 16];
        arr.copy_from_slice(&digest[..16]);
        let mut pick = u128::from_be_bytes(arr);
        let modulo = (self.total as u128);
        if modulo == 0 { return None; }
        pick = pick % modulo;

        // walk stakes
        let mut acc: u128 = 0;
        for info in &self.ordered {
            acc += info.stake as u128;
            if pick < acc {
                return Some(&info.validator);
            }
        }
        // fallback last
        Some(&self.ordered.last().unwrap().validator)
    }

    /// Select leader by slot using a round-robin fallback if no seed provided
    pub fn select_leader(&self, slot: u64) -> Option<&ValidatorId> {
        if self.ordered.is_empty() {
            return None;
        }
        let idx = (slot as usize) % self.ordered.len();
        Some(&self.ordered[idx].validator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_pos_register_and_select() {
        let mut pos = ProofOfStake::new();
        pos.register("alice".into(), 50);
        pos.register("bob".into(), 30);
        pos.register("carol".into(), 20);
        assert_eq!(pos.total_stake(), 100);
        let seed = b"random-seed";
        let leader = pos.select_leader_with_seed(seed).unwrap();
        assert!(["alice","bob","carol"].contains(&leader.as_str()));
    }
}
