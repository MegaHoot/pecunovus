use serde::{Serialize, Deserialize};
use sha2::{Digest, Sha256};

pub type Slot = u64;
pub type Epoch = u64;
pub type ValidatorId = String;

/// Vote cast by a validator for a proposal
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Vote {
    pub validator: ValidatorId,
    pub slot: Slot,
    pub block_hash: Vec<u8>,
    pub signature: Vec<u8>, // placeholder: in production this is ed25519 signature
}

/// Block proposal structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockProposal {
    pub proposer: ValidatorId,
    pub slot: Slot,
    pub block_hash: Vec<u8>,
    pub poh_hash: String, // PoH seed included for ordering
}

/// Finalized block info (very small footprint)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalizedBlock {
    pub slot: Slot,
    pub block_hash: Vec<u8>,
    pub proposer: ValidatorId,
}

/// utility: hash bytes to a Vec<u8>
pub fn hash_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().to_vec()
}
