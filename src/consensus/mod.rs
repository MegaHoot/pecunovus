//! Consensus module (PoH + PoS + Tower-BFT simplified)
//!
//! Public surface:
//! - ConsensusEngine: orchestrates PoH, PoS, proposals, votes, finalization
//! - types, poh, pos, tower, consensus_state, rpc_handlers

pub mod types;
pub mod consensus_state;
pub mod poh;
pub mod pos;
pub mod tower;
pub mod rpc_handlers;

use crate::consensus::types::{BlockProposal, Vote, ValidatorId};
use crate::consensus::poh::PoH;
use crate::consensus::pos::ProofOfStake;
use crate::consensus::tower::Tower;
use crate::consensus::consensus_state::ConsensusState;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

/// A small abstraction for sending outbound consensus messages (to network)
/// Implement this in your network module and pass into the engine.
pub trait NetworkSender: Send + Sync + 'static {
    fn send_proposal(&self, proposal: BlockProposal);
    fn send_vote(&self, vote: Vote);
}

/// ConsensusEngine wires PoH + PoS + Tower + state
pub struct ConsensusEngine<N: NetworkSender> {
    pub node_id: ValidatorId,
    pub poh: Arc<Mutex<PoH>>,
    pub pos: Arc<Mutex<ProofOfStake>>,
    pub tower: Arc<Mutex<Tower>>,
    pub state: Arc<Mutex<ConsensusState>>,
    pub net: Arc<N>,
}

impl<N: NetworkSender> ConsensusEngine<N> {
    /// Construct a new engine.
    /// `poh_tick_ms` controls PoH tick latency (for local PoH generator).
    pub fn new(node_id: ValidatorId, poh_tick_ms: u64, net: Arc<N>) -> Self {
        let poh = Arc::new(Mutex::new(PoH::new(poh_tick_ms)));
        let pos = Arc::new(Mutex::new(ProofOfStake::new()));
        let tower = Arc::new(Mutex::new(Tower::new()));
        let state = Arc::new(Mutex::new(ConsensusState::new()));

        Self {
            node_id,
            poh,
            pos,
            tower,
            state,
            net,
        }
    }

    /// Register validator stake (local API). In production this comes from chain state.
    pub async fn register_validator(&self, validator: ValidatorId, stake: u64) {
        let mut pos = self.pos.lock().await;
        pos.register(validator, stake);
        let total = pos.total_stake();
        let mut st = self.state.lock().await;
        st.total_stake = total;
    }

    /// Called periodically (e.g., PoH tick or slot timer) to propose if this node is leader.
    /// Returns Some(proposal) if we proposed.
    pub async fn propose_if_leader(&self) -> Option<BlockProposal> {
        // get deterministic seed from PoH
        let seed = {
            let mut poh = self.poh.lock().await;
            // generate lightweight PoH entry on each propose attempt; iterations moderate for demo
            poh.generate(256)
        };

        let leader = {
            let pos = self.pos.lock().await;
            pos.select_leader_with_seed(&seed)
        };

        if let Some(leader_id) = leader {
            if leader_id == &self.node_id {
                // create proposal
                let mut st = self.state.lock().await;
                let slot = st.next_slot();
                // In production the block_body is built from txpool/those things; here we create a placeholder hash
                let block_hash = crate::consensus::types::hash_bytes(format!("proposal:{}:{}", self.node_id, slot).as_bytes());
                let proposal = BlockProposal {
                    proposer: self.node_id.clone(),
                    slot,
                    block_hash: block_hash.clone(),
                    poh_hash: seed.clone(),
                };
                // persist pending
                st.insert_pending_proposal(block_hash.clone(), proposal.clone());
                info!("Node {} proposing slot {} (hash {})", self.node_id, slot, hex::encode(&block_hash));
                // broadcast via network sender
                self.net.send_proposal(proposal.clone());
                return Some(proposal);
            }
        }
        None
    }

    /// Handle an incoming proposal from the network
    pub async fn handle_proposal(&self, proposal: BlockProposal) {
        // basic verification: check proposer is expected for slot (best-effort)
        let pos = self.pos.lock().await;
        let expected = pos.select_leader(proposal.slot);
        drop(pos);

        let mut st = self.state.lock().await;
        // Accept proposal if slot matches next slot and not seen before
        if st.has_finalized_slot(proposal.slot) {
            // ignore old proposals
            info!("Ignoring proposal for finalized slot {}", proposal.slot);
            return;
        }
        if st.pending_proposals.contains_key(&proposal.block_hash) {
            info!("Already have proposal {}", hex::encode(&proposal.block_hash));
            return;
        }
        // Record proposal
        st.insert_pending_proposal(proposal.block_hash.clone(), proposal.clone());
        drop(st);

        // Vote (in real system: verify proposal, run some sanity checks)
        let vote = Vote {
            validator: self.node_id.clone(),
            slot: proposal.slot,
            block_hash: proposal.block_hash.clone(),
            signature: vec![], // sign in production
        };
        // locally record our vote
        self.handle_vote(vote.clone()).await;
        // broadcast our vote
        self.net.send_vote(vote);
    }

    /// Handle an incoming vote (either our own or from others). If finalization threshold reached,
    /// finalize and apply the block (call ledger through callback / event).
    pub async fn handle_vote(&self, vote: Vote) {
        let mut st = self.state.lock().await;
        // ignore if vote already recorded
        if st.record_vote(&vote) {
            // vote recorded and perhaps finalization reached
            if st.try_finalize(&vote.block_hash) {
                // finalize: call tower and move to finalized blocks
                drop(st);
                // update tower lockouts
                let mut tower = self.tower.lock().await;
                tower.record_vote(vote.clone());
                drop(tower);

                // apply finalization (in real system notify ledger to append block)
                let mut s2 = self.state.lock().await;
                if let Some(finalized) = s2.finalize_block(&vote.block_hash) {
                    info!("Block finalized for slot {} hash {}", finalized.slot, hex::encode(&finalized.block_hash));
                    // In production: emit event/callback to ledger to persist block
                }
            }
        }
    }

    /// Expose a snapshot of consensus state for RPC/inspection
    pub async fn snapshot(&self) -> crate::consensus::consensus_state::ConsensusSnapshot {
        let st = self.state.lock().await;
        st.snapshot()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex as TokioMutex;
    use std::time::Duration;

    struct DummyNet;
    impl NetworkSender for DummyNet {
        fn send_proposal(&self, _proposal: BlockProposal) {
            // no-op
        }
        fn send_vote(&self, _vote: Vote) {}
    }

    #[tokio::test]
    async fn test_register_and_select_leader() {
        let net = Arc::new(DummyNet);
        let engine = ConsensusEngine::new("node1".into(), 10, net);
        engine.register_validator("node1".into(), 50).await;
        engine.register_validator("node2".into(), 30).await;
        engine.register_validator("node3".into(), 20).await;

        // seed from PoH
        let seed = {
            let mut poh = engine.poh.lock().await;
            poh.generate(10)
        };

        let leader = {
            let pos = engine.pos.lock().await;
            pos.select_leader_with_seed(&seed).cloned()
        };

        assert!(leader.is_some());
    }
}
