//! Simple RPC facade for consensus queries. Integrate with your RPC server to expose status.

use crate::consensus::consensus_state::ConsensusSnapshot;
use crate::consensus::consensus_state::ConsensusState;

pub struct ConsensusRpcHandler {
    state: ConsensusState,
}

impl ConsensusRpcHandler {
    pub fn new(state: ConsensusState) -> Self {
        Self { state }
    }

    pub fn get_snapshot(&self) -> ConsensusSnapshot {
        self.state.snapshot()
    }

    pub fn get_slot(&self) -> u64 {
        self.state.current_slot
    }
}
