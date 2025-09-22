use thiserror::Error;

/// Unified error type for the blockchain
#[derive(Error, Debug)]
pub enum BlockchainError {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Consensus error: {0}")]
    ConsensusError(String),

    #[error("Ledger error: {0}")]
    LedgerError(String),

    #[error("Transaction error: {0}")]
    TransactionError(String),

    #[error("State error: {0}")]
    StateError(String),

    #[error("Crypto error: {0}")]
    CryptoError(String),

    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Convenience alias
pub type Result<T> = std::result::Result<T, BlockchainError>;
