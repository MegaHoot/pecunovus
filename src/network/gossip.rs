use crate::network::message::WireMessage;
use crate::network::peerstore::PeerStore;
use crate::network::connection::Connection;
use lru::LruCache;
use std::sync::Mutex;
use std::sync::Arc;
use tracing::debug;

/// Gossiper: small, configurable gossip broadcaster with dedup.
/// - dedup cache protects against re-broadcast storms
/// - fanout: send to `fanout` random peers (or all if fanout >= peers)
pub struct Gossiper {
    peerstore: PeerStore,
    dedup: Arc<Mutex<LruCache<Vec<u8>, ()>>>,
    /// fanout: how many peers to forward to when rebroadcasting
    pub fanout: usize,
}

impl Gossiper {
    pub fn new(peerstore: PeerStore, dedup_capacity: usize, fanout: usize) -> Self {
        Self {
            peerstore,
            dedup: Arc::new(Mutex::new(LruCache::new(dedup_capacity))),
            fanout,
        }
    }

    /// Broadcast a payload to peers (best-effort). `topic` is an application-level tag.
    /// `serialize` must be a WireMessage::Payload created by caller.
    pub async fn broadcast(&self, payload: WireMessage) {
        // dedup key = bincode(payload)
        match bincode::serialize(&payload) {
            Ok(bin) => {
                let mut dedup = self.dedup.lock().unwrap();
                if dedup.contains(&bin) {
                    debug!("gossip: duplicate payload; skipping");
                    return;
                }
                dedup.put(bin.clone(), ());
                drop(dedup);

                // forward to peers (fanout selection)
                let peers = self.peerstore.list_peers().await;
                let n = peers.len();
                if n == 0 {
                    return;
                }
                // naive: choose first `fanout` peers (replace with random sampling for production)
                let mut i = 0usize;
                for p in peers.into_iter() {
                    if i >= self.fanout { break; }
                    let addr = p.addr.clone();
                    let payload_clone = payload.clone();
                    tokio::spawn(async move {
                        if let Ok(stream) = tokio::net::TcpStream::connect(&addr).await {
                            if let Ok(mut conn) = Connection::spawn(stream, tokio::sync::mpsc::unbounded_channel().0).await {
                                let _ = conn.send(payload_clone).await;
                                conn.close();
                            }
                        } else {
                            debug!("gossip connect failed to {}", addr);
                        }
                    });
                    i += 1;
                }
            }
            Err(e) => {
                debug!("gossip serialize failed: {:?}", e);
            }
        }
    }
}
