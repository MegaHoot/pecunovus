//! TxPool: in-memory transaction pool with prioritization, dedup, TTL, and eviction.
//!
//! Data model:
//! - Tx: serialized transaction + metadata (fee, size, from, to, nonce)
//! - TxId: SHA-256 of serialized bytes (Vec<u8>)
//! - Under the hood: DashMap<TxId, TxEntry> for lookup + a min-heap (binary heap) for priority selection.
//!
//! Notes:
//! - This implementation keeps the priority heap and index lightly synchronized with a Mutex.
//! - For very high throughput, replace heap with a sharded priority queue.

use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use dashmap::DashMap;
use std::time::{Duration, Instant};
use sha2::{Sha256, Digest};
use std::cmp::Ordering;
use tokio::sync::Mutex;
use lru::LruCache;
use thiserror::Error;

/// Transaction ID type (SHA-256)
pub type TxId = Vec<u8>;

/// Priority numeric type: higher => more preferred
pub type Priority = f64;

/// Public TX model (serialize and sign in application layer)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tx {
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub fee: u64,
    pub nonce: u64,
    pub payload: Vec<u8>, // optional program invocation
}

impl Tx {
    pub fn serialized(&self) -> Vec<u8> {
        bincode::serialize(self).expect("serialize tx")
    }

    pub fn id(&self) -> TxId {
        let bin = self.serialized();
        let mut h = Sha256::new();
        h.update(&bin);
        h.finalize().to_vec()
    }

    /// approximate size for fee-per-byte metric
    pub fn size(&self) -> usize {
        self.serialized().len()
    }
}

/// Metadata tracked for each tx in pool
#[derive(Debug, Clone)]
pub struct TxMeta {
    pub id: TxId,
    pub inserted_at: Instant,
    pub priority: Priority,
    pub last_seen: Instant,
    pub ttl: Duration,
}

/// Error types
#[derive(Debug, Error)]
pub enum TxPoolError {
    #[error("duplicate tx")]
    Duplicate,
    #[error("pool full")]
    PoolFull,
    #[error("invalid tx")]
    Invalid,
}

/// Internal pool entry
struct TxEntry {
    tx: Tx,
    meta: TxMeta,
}

/// Heap item for priority queue (max-heap behavior using Ord)
#[derive(Clone)]
struct HeapItem {
    id: TxId,
    priority: Priority,
    inserted_at: Instant,
}

impl Eq for HeapItem {}
impl PartialEq for HeapItem {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.inserted_at == other.inserted_at && self.id == other.id
    }
}
impl PartialOrd for HeapItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for HeapItem {
    fn cmp(&self, other: &Self) -> Ordering {
        // higher priority first; on tie, earlier inserted first
        other.priority.partial_cmp(&self.priority).unwrap_or(Ordering::Equal)
            .then_with(|| self.inserted_at.cmp(&other.inserted_at))
    }
}

/// The TxPool main struct
pub struct TxPool {
    // fast lookup: txid -> TxEntry
    entries: DashMap<Vec<u8>, Arc<TxEntry>>,
    // priority heap protected by mutex (simple sync)
    heap: Mutex<std::collections::BinaryHeap<HeapItem>>,
    // LRU eviction for low-priority removals (stores txid)
    lru: Mutex<LruCache<Vec<u8>, ()>>,
    // configuration
    pub max_size: usize,
    pub ttl: Duration,
}

impl TxPool {
    /// Create new pool
    pub fn new(max_size: usize, ttl: Duration, lru_capacity: usize) -> Self {
        Self {
            entries: DashMap::new(),
            heap: Mutex::new(std::collections::BinaryHeap::new()),
            lru: Mutex::new(LruCache::new(lru_capacity)),
            max_size,
            ttl,
        }
    }

    /// Compute priority (fee per byte). You can replace with any policy.
    fn compute_priority(tx: &Tx) -> Priority {
        let fee = tx.fee as f64;
        let size = tx.size() as f64;
        if size == 0.0 { return fee; }
        fee / size
    }

    /// Insert transaction after validation by caller.
    /// Returns TxMeta or error (duplicate / pool full).
    pub async fn insert(&self, tx: Tx) -> Result<TxMeta, TxPoolError> {
        let txid = tx.id();
        // dedup
        if self.entries.contains_key(&txid) {
            return Err(TxPoolError::Duplicate);
        }

        // admission control: pool size
        if self.entries.len() >= self.max_size {
            // try to evict lower priority entries
            if !self.evict_low_priority().await {
                return Err(TxPoolError::PoolFull);
            }
        }

        let prio = Self::compute_priority(&tx);
        let now = Instant::now();
        let meta = TxMeta { id: txid.clone(), inserted_at: now, priority: prio, last_seen: now, ttl: self.ttl };
        let entry = TxEntry { tx: tx.clone(), meta: meta.clone() };
        let arc = Arc::new(entry);

        self.entries.insert(txid.clone(), arc);
        // push to priority heap
        let mut heap = self.heap.lock().await;
        heap.push(HeapItem { id: txid.clone(), priority: prio, inserted_at: now });
        drop(heap);
        // touch lru
        let mut lru = self.lru.lock().await;
        lru.put(txid.clone(), ());
        drop(lru);
        Ok(meta)
    }

    /// Try to evict one low-priority entry (LRU) to free space.
    /// Returns true if eviction occurred.
    async fn evict_low_priority(&self) -> bool {
        let mut lru = self.lru.lock().await;
        if let Some((txid, _)) = lru.pop_lru() {
            // remove from entries
            self.entries.remove(&txid);
            return true;
        }
        false
    }

    /// Pop up to `limit` highest-priority transactions (consensus/leader selection).
    /// Returns Vec<Tx> in descending priority order.
    pub async fn pop_priority(&self, limit: usize) -> Vec<Tx> {
        let mut selected = Vec::new();
        let mut heap = self.heap.lock().await;
        while selected.len() < limit {
            if let Some(item) = heap.pop() {
                // lookup id in entries (it may have been removed)
                if let Some(entry) = self.entries.remove(&item.id) {
                    // entry.1 is Arc<TxEntry>
                    let arc_entry = entry.1;
                    selected.push(arc_entry.tx.clone());
                    // also remove from LRU
                    let mut lru = self.lru.lock().await;
                    lru.pop(&item.id);
                    drop(lru);
                } else {
                    // skip stale
                    continue;
                }
            } else {
                break;
            }
        }
        drop(heap);
        selected
    }

    /// Get tx by id
    pub fn get(&self, txid: &TxId) -> Option<Tx> {
        self.entries.get(txid).map(|arc| arc.value().tx.clone())
    }

    /// Remove a transaction (e.g., after it's included)
    pub async fn remove(&self, txid: &TxId) {
        self.entries.remove(txid);
        // best-effort remove from lru and heap (heap removal is O(n), we avoid; heap will skip stale ids on pop)
        let mut lru = self.lru.lock().await;
        lru.pop(txid);
        drop(lru);
    }

    /// Cleanup expired transactions by TTL
    pub async fn gc_ttl(&self) {
        let now = Instant::now();
        let keys: Vec<Vec<u8>> = self.entries.iter()
            .filter_map(|r| {
                let e = r.value();
                if now.duration_since(e.meta.inserted_at) > e.meta.ttl {
                    Some(e.meta.id.clone())
                } else {
                    None
                }
            })
            .collect();

        for k in keys {
            self.entries.remove(&k);
            let mut lru = self.lru.lock().await;
            lru.pop(&k);
            drop(lru);
        }
    }

    /// Pool size
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_insert_and_pop_priority() {
        let pool = TxPool::new(100, Duration::from_secs(60), 100);
        let tx1 = Tx { from: "a".into(), to: "b".into(), amount: 10, fee: 100, nonce: 1, payload: vec![] };
        let tx2 = Tx { from: "c".into(), to: "d".into(), amount: 5, fee: 10, nonce: 1, payload: vec![] };

        pool.insert(tx1.clone()).await.unwrap();
        pool.insert(tx2.clone()).await.unwrap();

        let popped = pool.pop_priority(2).await;
        // tx1 has higher fee-per-byte, so should be first
        assert_eq!(popped.len(), 2);
        assert_eq!(popped[0].from, "a");
    }

    #[tokio::test]
    async fn test_dedup() {
        let pool = TxPool::new(10, Duration::from_secs(60), 10);
        let tx = Tx { from: "a".into(), to: "b".into(), amount: 1, fee: 1, nonce: 1, payload: vec![] };
        pool.insert(tx.clone()).await.unwrap();
        let res = pool.insert(tx.clone()).await;
        assert!(matches!(res.unwrap_err(), TxPoolError::Duplicate));
    }

    #[tokio::test]
    async fn test_ttl_gc() {
        let pool = TxPool::new(10, Duration::from_millis(10), 10);
        let tx = Tx { from: "a".into(), to: "b".into(), amount: 1, fee: 1, nonce: 1, payload: vec![] };
        pool.insert(tx.clone()).await.unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;
        pool.gc_ttl().await;
        assert_eq!(pool.len(), 0);
    }
}
