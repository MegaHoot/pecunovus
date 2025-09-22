use std::time::{Instant, Duration};
use std::collections::HashMap;
use tokio::sync::RwLock;
use std::sync::Arc;

/// Peer metadata tracked by PeerStore
#[derive(Clone, Debug)]
pub struct PeerInfo {
    pub node_id: String,
    pub addr: String,
    pub last_seen: Instant,
    pub ban: Option<Instant>, // until when banned
}

impl PeerInfo {
    pub fn healthy(&self) -> bool {
        if let Some(until) = self.ban {
            Instant::now() > until
        } else {
            true
        }
    }
}

/// In-memory peerstore with a simple API. Replace with RocksDB / sled-backed persister if needed.
#[derive(Clone, Debug)]
pub struct PeerStore {
    inner: Arc<RwLock<HashMap<String, PeerInfo>>>, // node_id -> PeerInfo
}

impl PeerStore {
    pub fn new() -> Self {
        Self { inner: Arc::new(RwLock::new(HashMap::new())) }
    }

    pub async fn add_peer(&self, node_id: String, addr: String) {
        let mut map = self.inner.write().await;
        map.insert(node_id.clone(), PeerInfo { node_id, addr, last_seen: Instant::now(), ban: None });
    }

    pub async fn update_seen(&self, node_id: &str) {
        let mut map = self.inner.write().await;
        if let Some(p) = map.get_mut(node_id) {
            p.last_seen = Instant::now();
        }
    }

    pub async fn remove_peer(&self, node_id: &str) {
        let mut map = self.inner.write().await;
        map.remove(node_id);
    }

    pub async fn list_peers(&self) -> Vec<PeerInfo> {
        let map = self.inner.read().await;
        map.values().cloned().collect()
    }

    pub async fn gc(&self, timeout: Duration) {
        let mut map = self.inner.write().await;
        map.retain(|_, v| v.last_seen.elapsed() < timeout);
    }

    /// Ban a peer for a given duration
    pub async fn ban_peer(&self, node_id: &str, dur: Duration) {
        let mut map = self.inner.write().await;
        if let Some(p) = map.get_mut(node_id) {
            p.ban = Some(Instant::now() + dur);
        }
    }
}
