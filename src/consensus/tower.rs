//! Tower BFT simplified: track votes and lockouts, provide basic finality tracking.
//!
//! This is a simplified model: Tower records vote slots and can be extended with lockout
//! doubling rules, vote expiry, and slashing detection.

use crate::consensus::types::{Vote};
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct Tower {
    /// per-validator votes recorded (validator -> set of slots they voted)
    pub votes_by_validator: HashMap<String, HashSet<u64>>,
}

impl Tower {
    pub fn new() -> Self {
        Self {
            votes_by_validator: HashMap::new(),
        }
    }

    /// Record an observed vote
    pub fn record_vote(&mut self, vote: Vote) {
        let entry = self.votes_by_validator.entry(vote.validator.clone()).or_insert_with(HashSet::new);
        entry.insert(vote.slot);
    }

    /// Check basic liveness: has validator voted at slot?
    pub fn has_voted(&self, validator: &str, slot: u64) -> bool {
        self.votes_by_validator.get(validator).map_or(false, |s| s.contains(&slot))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::types::Vote;

    #[test]
    fn test_tower_record() {
        let mut t = Tower::new();
        let v = Vote { validator: "alice".into(), slot: 1, block_hash: vec![], signature: vec![] };
        t.record_vote(v);
        assert!(t.has_voted("alice", 1));
    }
}
