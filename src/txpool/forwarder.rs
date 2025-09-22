//! Tx forwarder: takes prioritized transactions from pool and forwards to leader(s) or gossips them.
//!
//! Modes:
//! - Forward to a single leader (Gulf Stream-like push)
//! - Gossip to peers for propagation
//!
//! The forwarder uses a pluggable `NetworkSender` trait to send bytes to peers.
//! Forwarder runs an internal loop (tokio task) pulling txs and forwarding with backpressure.

use crate::txpool::pool::{TxPool, Tx};
use anyhow::Result;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{info, debug};

/// Network sender abstraction (implement in your network module)
#[async_trait::async_trait]
pub trait NetworkSender: Send + Sync + 'static {
    async fn send_to_peer(&self, peer_addr: &str, topic: &str, data: Vec<u8>) -> Result<()>;
    async fn broadcast(&self, topic: &str, data: Vec<u8>) -> Result<()>;
}

/// Forwarding configuration
pub struct ForwardConfig {
    pub leader_addr: Option<String>,
    pub gossip: bool,
    pub batch_size: usize,
    pub poll_interval_ms: u64,
}

impl Default for ForwardConfig {
    fn default() -> Self {
        Self { leader_addr: None, gossip: true, batch_size: 64, poll_interval_ms: 100 }
    }
}

/// TxForwarder: pulls txs from pool and forwards
pub struct TxForwarder<N: NetworkSender> {
    pool: Arc<TxPool>,
    net: Arc<N>,
    cfg: ForwardConfig,
    shutdown: tokio::sync::watch::Receiver<bool>,
}

impl<N: NetworkSender> TxForwarder<N> {
    pub fn new(pool: Arc<TxPool>, net: Arc<N>, cfg: ForwardConfig, shutdown: tokio::sync::watch::Receiver<bool>) -> Self {
        Self { pool, net, cfg, shutdown }
    }

    /// Start the forwarding loop (spawn this on tokio)
    pub async fn run(mut self) {
        loop {
            // check shutdown
            if *self.shutdown.borrow() {
                info!("txforwarder shutdown signal received");
                return;
            }

            // batch up txs
            let txs = self.pool.pop_priority(self.cfg.batch_size).await;
            if txs.is_empty() {
                sleep(Duration::from_millis(self.cfg.poll_interval_ms)).await;
                continue;
            }

            // serialize batch (for demo we serialize individual txs and send)
            for tx in txs.into_iter() {
                let bytes = bincode::serialize(&tx).expect("serialize tx");
                // forward to leader preferentially
                if let Some(ref leader) = self.cfg.leader_addr {
                    let _ = self.net.send_to_peer(leader, "tx", bytes.clone()).await;
                }
                // optionally gossip as fallback
                if self.cfg.gossip {
                    let _ = self.net.broadcast("tx", bytes.clone()).await;
                }
            }

            // small backoff
            sleep(Duration::from_millis(1)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use crate::txpool::pool::TxPool;
    use std::time::Duration;

    struct DummyNet {
        pub counter: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl NetworkSender for DummyNet {
        async fn send_to_peer(&self, _peer_addr: &str, _topic: &str, _data: Vec<u8>) -> Result<()> {
            self.counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        async fn broadcast(&self, _topic: &str, _data: Vec<u8>) -> Result<()> {
            self.counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_forwarder_runs_and_forwards() {
        let pool = Arc::new(TxPool::new(100, Duration::from_secs(60), 100));
        let (tx, rx) = tokio::sync::watch::channel(false);
        let counter = Arc::new(AtomicUsize::new(0));
        let net = Arc::new(DummyNet { counter: counter.clone() });
        let cfg = ForwardConfig { leader_addr: Some("127.0.0.1:1".into()), gossip: true, batch_size: 10, poll_interval_ms: 10 };
        let forwarder = TxForwarder::new(pool.clone(), net, cfg, rx);

        // insert some txs
        for i in 0..5 {
            let tx = crate::txpool::pool::Tx { from: format!("a{}", i), to: "b".into(), amount: 1, fee: 10, nonce: 0, payload: vec![] };
            pool.insert(tx).await.unwrap();
        }

        // spawn forwarder
        let handle = tokio::spawn(async move {
            forwarder.run().await;
        });

        // wait a bit to let it forward
        tokio::time::sleep(Duration::from_millis(200)).await;
        // signal shutdown
        let _ = tx.send(true);
        let _ = handle.await;

        assert!(counter.load(Ordering::SeqCst) > 0);
    }
}
