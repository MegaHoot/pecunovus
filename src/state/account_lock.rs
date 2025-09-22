//! Sharded async account locking for parallel transaction execution.
//!
//! Uses `tokio::sync::Mutex` per account and `OwnedMutexGuard` to hold locks across await
//! points and across tasks. Locks are acquired in deterministic sorted order to prevent deadlocks.
//!
//! API:
//! - `AccountLocks::new(shard_count)`
//! - `AccountLocks::acquire(keys: Vec<AccountKey>) -> LockGuard` (async)
//! - `LockGuard` holds the OwnedMutexGuards and releases them on Drop.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex as TokioMutex, OwnedMutexGuard};
use crate::state::account_db::AccountKey;
use twox_hash::XxHash64;
use std::hash::Hasher;

/// Default number of shards (tweak for CPU/core count).
pub const DEFAULT_SHARDS: usize = 256;

/// LockGuard holds the actual OwnedMutexGuard for each acquired account lock.
/// The guards are released when LockGuard is dropped.
pub struct LockGuard {
    // vector of owned guards; drop releases locks
    guards: Vec<OwnedMutexGuard<()>>,
}

impl LockGuard {
    pub fn len(&self) -> usize {
        self.guards.len()
    }
}

/// AccountLocks: shards -> per-shard map of account_key -> Arc<tokio::Mutex<()>>
/// We store Arc<Mutex<()>> per account so the same Mutex is reused when different tasks request locks.
#[derive(Clone)]
pub struct AccountLocks {
    shards: Arc<Vec<TokioMutex<HashMap<AccountKey, Arc<TokioMutex<()>>>>>>,
    shard_count: usize,
}

impl AccountLocks {
    /// Create new AccountLocks with given shard count.
    pub fn new(shard_count: usize) -> Self {
        let mut v = Vec::with_capacity(shard_count);
        for _ in 0..shard_count {
            v.push(TokioMutex::new(HashMap::new()));
        }
        Self {
            shards: Arc::new(v),
            shard_count,
        }
    }

    /// Hash-based shard selection
    fn shard_for(&self, key: &AccountKey) -> usize {
        let mut hasher = XxHash64::default();
        hasher.write(key.as_bytes());
        (hasher.finish() as usize) % self.shard_count
    }

    /// Acquire locks for a set of keys.
    /// Ordering: keys are sorted (lexicographically) to avoid deadlocks across concurrent acquirers.
    /// Returns a LockGuard which holds acquired OwnedMutexGuards until dropped.
    pub async fn acquire(&self, mut keys: Vec<AccountKey>) -> LockGuard {
        // sort keys -> deterministic acquisition order
        keys.sort();

        // We'll collect OwnedMutexGuard in guards.
        let mut guards: Vec<OwnedMutexGuard<()>> = Vec::with_capacity(keys.len());

        // For each key, find or create its per-key Arc<Mutex<()>>, then call lock_owned().await to acquire.
        for key in keys {
            let sid = self.shard_for(&key);
            // scope the borrow of shard map to avoid holding across await
            let key_mutex_arc = {
                let mut shard_map = self.shards[sid].lock().await;
                if let Some(m) = shard_map.get(&key) {
                    m.clone()
                } else {
                    let m = Arc::new(TokioMutex::new(()));
                    shard_map.insert(key.clone(), m.clone());
                    m
                }
            };
            // Acquire the owned lock (await)
            let guard = key_mutex_arc.lock_owned().await;
            guards.push(guard);
            // continue to next key; we hold the lock until LockGuard dropped
        }

        LockGuard { guards }
    }

    /// Try to acquire locks for keys without awaiting if not immediately available.
    /// Returns Some(LockGuard) if all locks were acquired immediately, None otherwise.
    /// This uses `try_lock` on tokio::Mutex is not available; so we emulate by attempting to `try_lock_owned`
    /// via `Arc::try_unwrap` - not practical. Therefore we omit try_acquire (or implement using synchronous Mutex).
    /// For now, prefer `acquire(...).await`.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::task;
    use futures::future::join_all;

    #[tokio::test]
    async fn test_acquire_non_conflicting() {
        let locks = AccountLocks::new(16);
        let k1 = "a".to_string();
        let k2 = "b".to_string();

        // acquire k1 in a task and hold for a short duration
        let locks_clone = locks.clone();
        let t1 = task::spawn(async move {
            let guard = locks_clone.acquire(vec![k1.clone()]).await;
            assert_eq!(guard.len(), 1);
            // hold for 100ms while other tries to acquire k2
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            drop(guard);
        });

        // concurrently acquire k2 (different key) - should not block
        let locks_clone2 = locks.clone();
        let t2 = task::spawn(async move {
            let g = locks_clone2.acquire(vec![k2.clone()]).await;
            assert_eq!(g.len(), 1);
            drop(g);
        });

        let _ = join_all(vec![t1, t2]).await;
    }

    #[tokio::test]
    async fn test_acquire_conflicting() {
        let locks = AccountLocks::new(16);
        let k = "shared".to_string();

        // task1 acquires and holds for 200ms
        let locks1 = locks.clone();
        let t1 = task::spawn(async move {
            let _guard = locks1.acquire(vec![k.clone()]).await;
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            // guard dropped here
        });

        // give t1 a head start
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        // task2 tries to acquire same key - will await until t1 releases
        let locks2 = locks.clone();
        let t2 = task::spawn(async move {
            let _g2 = locks2.acquire(vec![k.clone()]).await;
            // if we reach here, lock was acquired after t1 dropped
        });

        let _ = join_all(vec![t1, t2]).await;
    }
}
