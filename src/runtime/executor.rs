//! Runtime Executor
//!
//! Executes transactions in parallel using AccountLocks + AccountCache.
//! Produces Receipts and commits to the account state.

use crate::state::{AccountCache, AccountLocks};
use crate::state::account_db::AccountKey;
use anyhow::Result;
use std::sync::Arc;
use tokio::task::JoinHandle;
use serde::{Serialize, Deserialize};

/// Simple transaction model (transfer + nonce).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub from: AccountKey,
    pub to: AccountKey,
    pub amount: u64,
    pub nonce: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub tx: Transaction,
    pub success: bool,
    pub err: Option<String>,
    pub post_balances: Option<(u64, u64)>, // (from, to)
}

pub struct Executor {
    pub cache: AccountCache,
    pub locks: AccountLocks,
}

impl Executor {
    pub fn new(cache: AccountCache, locks: AccountLocks) -> Self {
        Self { cache, locks }
    }

    /// Execute a batch of transactions in parallel.
    pub async fn execute_transactions(&self, txs: Vec<Transaction>) -> Vec<Receipt> {
        let mut handles: Vec<JoinHandle<Receipt>> = Vec::with_capacity(txs.len());
        let cache = self.cache.clone();
        let locks = self.locks.clone();

        for tx in txs.into_iter() {
            let cache_cl = cache.clone();
            let locks_cl = locks.clone();

            let handle = tokio::spawn(async move {
                let mut keys = vec![tx.from.clone(), tx.to.clone()];
                keys.sort();
                keys.dedup();

                let guard = locks_cl.acquire(keys.clone()).await;

                let mut err = None;
                let mut success = false;
                let mut post_balances = None;

                let from_acc_opt = cache_cl.get(&tx.from).unwrap_or(None);
                let to_acc_opt = cache_cl.get(&tx.to).unwrap_or(None);

                let mut from_acc = from_acc_opt.unwrap_or_else(|| crate::state::account_db::Account::new(0, "system", vec![]));
                let mut to_acc = to_acc_opt.unwrap_or_else(|| crate::state::account_db::Account::new(0, "system", vec![]));

                if from_acc.lamports < tx.amount {
                    err = Some("insufficient funds".to_string());
                } else {
                    from_acc.lamports = from_acc.lamports.saturating_sub(tx.amount);
                    to_acc.lamports = to_acc.lamports.saturating_add(tx.amount);
                    let _ = cache_cl.insert(tx.from.clone(), from_acc.clone());
                    let _ = cache_cl.insert(tx.to.clone(), to_acc.clone());
                    success = true;
                    post_balances = Some((from_acc.lamports, to_acc.lamports));
                }

                drop(guard);

                Receipt { tx, success, err, post_balances }
            });

            handles.push(handle);
        }

        let mut receipts: Vec<Receipt> = Vec::with_capacity(handles.len());
        for h in handles {
            match h.await {
                Ok(r) => receipts.push(r),
                Err(e) => {
                    receipts.push(Receipt {
                        tx: Transaction { from: "".into(), to: "".into(), amount: 0, nonce: 0 },
                        success: false,
                        err: Some(format!("task error: {:?}", e)),
                        post_balances: None,
                    });
                }
            }
        }

        if let Err(e) = self.cache.flush() {
            tracing::error!("cache flush failed: {:?}", e);
        }

        receipts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::account_db::InMemAccountStore;

    #[tokio::test]
    async fn test_parallel_exec() {
        let store = Arc::new(InMemAccountStore::new());
        let cache = AccountCache::new(store.clone());
        let locks = AccountLocks::new(16);
        let exec = Executor::new(cache.clone(), locks.clone());

        let a = "alice".to_string();
        let b = "bob".to_string();
        cache.insert(a.clone(), crate::state::account_db::Account::new(100, "system", vec![])).unwrap();
        cache.insert(b.clone(), crate::state::account_db::Account::new(50, "system", vec![])).unwrap();

        let txs = vec![
            Transaction { from: a.clone(), to: b.clone(), amount: 30, nonce: 1 },
            Transaction { from: a.clone(), to: b.clone(), amount: 40, nonce: 2 },
        ];

        let receipts = exec.execute_transactions(txs).await;
        assert_eq!(receipts.len(), 2);

        let a_after = cache.get(&a).unwrap().unwrap();
        let b_after = cache.get(&b).unwrap().unwrap();
        assert_eq!(a_after.lamports, 30);
        assert_eq!(b_after.lamports, 50 + 30 + 40);
    }
}
