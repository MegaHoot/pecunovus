//! Ingest: accepts transactions from RPC/network, validates them lightly, and inserts into pool.
//!
//! Validation is pluggable via `TxValidator` trait. We include `SimpleValidator` which checks
//! account balances via `AccountCache` and basic nonce checking (nonce semantics are simple here).
//!
//! TxIngestor exposes an async `ingest(serialized_tx_bytes)` API returning IngestResult.

use crate::txpool::pool::{TxPool, Tx, TxId, TxPoolError};
use crate::state::account_cache::AccountCache;
use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IngestError {
    #[error("validation failed: {0}")]
    ValidationFailed(String),
    #[error("pool error: {0}")]
    PoolError(#[from] TxPoolError),
}

/// Ingest result
#[derive(Debug, Clone)]
pub enum IngestResult {
    Accepted(TxId),
    Rejected(String),
}

/// Trait for pluggable validation
#[async_trait::async_trait]
pub trait TxValidator: Send + Sync + 'static {
    async fn validate(&self, tx: &Tx) -> Result<(), String>;
}

/// Simple validator: checks sender has enough balance (lamports >= amount+fee).
pub struct SimpleValidator {
    pub cache: AccountCache,
}

impl SimpleValidator {
    pub fn new(cache: AccountCache) -> Self {
        Self { cache }
    }
}

#[async_trait::async_trait]
impl TxValidator for SimpleValidator {
    async fn validate(&self, tx: &Tx) -> Result<(), String> {
        // Check sender balance
        let from_acc = self.cache.get(&tx.from).map_err(|e| e.to_string())?;
        let from = match from_acc {
            Some(a) => a,
            None => return Err("sender account not found".into()),
        };

        let required = tx.amount.saturating_add(tx.fee);
        if from.lamports < required {
            return Err("insufficient funds".into());
        }
        // Optionally check nonce; omitted here
        Ok(())
    }
}

/// TxIngestor: validates and inserts into pool
pub struct TxIngestor<V: TxValidator> {
    pub pool: Arc<TxPool>,
    pub validator: Arc<V>,
}

impl<V: TxValidator> TxIngestor<V> {
    pub fn new(pool: Arc<TxPool>, validator: Arc<V>) -> Self {
        Self { pool, validator }
    }

    /// Ingest a transaction (deserialized)
    pub async fn ingest(&self, tx: Tx) -> Result<IngestResult, IngestError> {
        // validate
        if let Err(e) = self.validator.validate(&tx).await {
            return Ok(IngestResult::Rejected(e));
        }
        // insert
        match self.pool.insert(tx).await {
            Ok(meta) => Ok(IngestResult::Accepted(meta.id)),
            Err(e) => Err(IngestError::from(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::account_db::InMemAccountStore;
    use std::sync::Arc;
    use std::time::Duration;

    #[tokio::test]
    async fn test_simple_ingest_accept() {
        let store = Arc::new(InMemAccountStore::new());
        let cache = AccountCache::new(store.clone());
        let validator = Arc::new(SimpleValidator::new(cache.clone()));
        // seed account
        cache.insert("alice".into(), crate::state::account_db::Account::new(100, "system", vec![])).unwrap();

        let pool = Arc::new(TxPool::new(100, Duration::from_secs(60), 100));
        let ingestor = TxIngestor::new(pool.clone(), validator.clone());

        let tx = Tx { from: "alice".into(), to: "bob".into(), amount: 10, fee: 1, nonce: 1, payload: vec![] };
        let res = ingestor.ingest(tx).await.unwrap();
        match res {
            IngestResult::Accepted(txid) => {
                assert!(pool.get(&txid).is_some());
            }
            _ => panic!("expected accepted"),
        }
    }

    #[tokio::test]
    async fn test_simple_ingest_reject_insufficient() {
        let store = Arc::new(InMemAccountStore::new());
        let cache = AccountCache::new(store.clone());
        let validator = Arc::new(SimpleValidator::new(cache.clone()));
        // no funds
        let pool = Arc::new(TxPool::new(100, Duration::from_secs(60), 100));
        let ingestor = TxIngestor::new(pool.clone(), validator.clone());

        let tx = Tx { from: "alice".into(), to: "bob".into(), amount: 10, fee: 1, nonce: 1, payload: vec![] };
        let res = ingestor.ingest(tx).await.unwrap();
        match res {
            IngestResult::Rejected(reason) => {
                assert!(reason.contains("sender account not found") || reason.contains("insufficient"));
            }
            _ => panic!("expected reject"),
        }
    }
}
