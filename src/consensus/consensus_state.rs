//! ConsensusState manages pending proposals, votes, and finalization bookkeeping.
//!
//! This module intentionally keeps logic deterministic and simple for testing — production
//! should persist vote history and pending proposals to disk and handle forks, reorgs, etc.

use crate::consensus::types::{Slot, Epoch, Vote, BlockProposal, FinalizedBlock};
use std::collections::{HashMap, HashSet};

/// Snapshot representation for RPC/debugging
#[derive(Debug, Clone)]
pub struct ConsensusSnapshot {
    pub epoch: Epoch,
    pub slot: Slot,
    pub total_stake: u64,
    pub pending_proposals: Vec<(Slot, Vec<u8>)>,
    pub finalized: Vec<FinalizedBlock>,
}

/// ConsensusState holds live consensus information
#[derive(Debug)]
pub struct ConsensusState {
    pub current_epoch: Epoch,
    pub current_slot: Slot,
    pub total_stake: u64,

    /// pending proposals by block_hash -> proposal
    pub pending_proposals: HashMap<Vec<u8>, BlockProposal>,

    /// votes: block_hash -> set of validators who voted, and yes_stake aggregated
    pub votes: HashMap<Vec<u8>, (HashSet<String>, u64)>,

    /// finalized blocks in order
    pub finalized: Vec<FinalizedBlock>,
}

impl ConsensusState {
    pub fn new() -> Self {
        Self {
            current_epoch: 0,
            current_slot: 0,
            total_stake: 0,
            pending_proposals: HashMap::new(),
            votes: HashMap::new(),
            finalized: vec![],
        }
    }

    /// Get next slot (increments slot counter)
    pub fn next_slot(&mut self) -> Slot {
        self.current_slot += 1;
        self.current_slot
    }

    pub fn insert_pending_proposal(&mut self, block_hash: Vec<u8>, proposal: BlockProposal) {
        self.pending_proposals.insert(block_hash, proposal);
    }

    /// record_vote returns true if vote was newly recorded (not duplicate)
    pub fn record_vote(&mut self, vote: &Vote) -> bool {
        let key = &vote.block_hash;
        let voters_stake = self.votes.entry(key.clone()).or_insert_with(|| (HashSet::new(), 0u64));
        if voters_stake.0.contains(&vote.validator) {
            return false; // duplicate vote
        }
        // For simplicity, our Vote struct does not include stake; in practice we need weight.
        // Here we treat each vote as weight 1 and require 2/3 of number-of-validators (not stake).
        voters_stake.0.insert(vote.validator.clone());
        voters_stake.1 = voters_stake.1.saturating_add(1); // placeholder weight
        true
    }

    /// Try to finalize block identified by block_hash. Returns true if finalization reached.
    /// In this simplified model, we require yes_votes_count >= (2/3 * total_stake) where total_stake
    /// is expected to be set by the PoS registrar (in register_validator).
    pub fn try_finalize(&self, block_hash: &Vec<u8>) -> bool {
        if let Some((voters, yes_weight)) = self.votes.get(block_hash) {
            // Note: here yes_weight is not real stake; in production vote includes stake or network must map voter->stake.
            let yes = *yes_weight as u128;
            let total = self.total_stake as u128;
            if total == 0 {
                return false;
            }
            // finalization condition: yes * 3 >= total * 2 (i.e., yes >= 2/3 total)
            return yes * 3 >= total * 2;
        }
        false
    }

    pub fn has_finalized_slot(&self, slot: Slot) -> bool {
        self.finalized.iter().any(|f| f.slot == slot)
    }

    /// finalize_block consumes a pending proposal and moves it to finalized list.
    pub fn finalize_block(&mut self, block_hash: &Vec<u8>) -> Option<FinalizedBlock> {
        if let Some(proposal) = self.pending_proposals.remove(block_hash) {
            let finalized = FinalizedBlock {
                slot: proposal.slot,
                block_hash: block_hash.clone(),
                proposer: proposal.proposer,
            };
            self.finalized.push(finalized.clone());
            Some(finalized)
        } else {
            None
        }
    }

    pub fn snapshot(&self) -> ConsensusSnapshot {
        ConsensusSnapshot {
            epoch: self.current_epoch,
            slot: self.current_slot,
            total_stake: self.total_stake,
            pending_proposals: self.pending_proposals.iter().map(|(h, p)| (p.slot, h.clone())).collect(),
            finalized: self.finalized.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::types::BlockProposal;

    #[test]
    fn test_insert_and_finalize_flow() {
        let mut st = ConsensusState::new();
        st.total_stake = 3;
        let hash = vec![1,2,3];
        let prop = BlockProposal {
            proposer: "alice".into(),
            slot: 1,
            block_hash: hash.clone(),
            poh_hash: "seed".into(),
        };
        st.insert_pending_proposal(hash.clone(), prop);
        // record votes from three different validators (we treat each vote as weight 1)
        let v1 = crate::consensus::types::Vote { validator: "a".into(), slot: 1, block_hash: hash.clone(), signature: vec![] };
        let v2 = crate::consensus::types::Vote { validator: "b".into(), slot: 1, block_hash: hash.clone(), signature: vec![] };
        let v3 = crate::consensus::types::Vote { validator: "c".into(), slot: 1, block_hash: hash.clone(), signature: vec![] };

        assert!(st.record_vote(&v1));
        assert!(st.record_vote(&v2));
        assert!(st.record_vote(&v3));
        assert!(st.try_finalize(&hash));
        let fin = st.finalize_block(&hash);
        assert!(fin.is_some());
    }
}
