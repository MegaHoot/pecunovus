use serde::{Deserialize, Serialize};
use anyhow::Result;
use async_trait::async_trait;
use crate::consensus;
use crate::ledger;
use crate::txpool;
use crate::state;

/// Trait describing dependencies the RPC handlers require.
/// Implement this trait in your node wiring layer and pass into RpcServer.
#[async_trait]
pub trait RpcDeps: Send + Sync + 'static {
    /// Get a snapshot of consensus state for status calls
    async fn consensus_snapshot(&self) -> crate::consensus::consensus_state::ConsensusSnapshot;

    /// Submit a signed transaction (raw deserialized TX object)
    async fn submit_transaction(&self, tx: crate::txpool::pool::Tx) -> Result<txpool::ingest::IngestResult>;

    /// Get block bytes by slot
    async fn get_block(&self, slot: u64) -> Result<Option<Vec<u8>>>;

    /// Get account by key
    async fn get_account(&self, key: &str) -> Result<Option<crate::state::account_db::Account>>;

    /// Get mempool size
    async fn mempool_size(&self) -> usize;
}

/// A small wrapper that calls into RpcDeps to handle requests
pub struct RpcHandler<D: RpcDeps> {
    deps: std::sync::Arc<D>,
}

impl<D: RpcDeps> RpcHandler<D> {
    pub fn new(deps: std::sync::Arc<D>) -> Self {
        Self { deps }
    }

    /// Return a JSON-serializable status object
    pub async fn status(&self) -> Result<serde_json::Value> {
        let snap = self.deps.consensus_snapshot().await;
        Ok(serde_json::json!({
            "slot": snap.slot,
            "epoch": snap.epoch,
            "total_stake": snap.total_stake,
            "finalized": snap.finalized.len()
        }))
    }

    /// JSON-RPC method: get_block
    pub async fn get_block(&self, slot: u64) -> Result<Option<Vec<u8>>> {
        self.deps.get_block(slot).await
    }

    /// JSON-RPC method: submit_tx
    pub async fn submit_tx(&self, tx: crate::txpool::pool::Tx) -> Result<txpool::ingest::IngestResult> {
        let res = self.deps.submit_transaction(tx).await?;
        Ok(res)
    }

    /// REST: get account
    pub async fn get_account(&self, key: String) -> Result<Option<crate::state::account_db::Account>> {
        self.deps.get_account(&key).await
    }

    /// REST: mempool size
    pub async fn mempool_size(&self) -> Result<usize> {
        Ok(self.deps.mempool_size())
    }
}
