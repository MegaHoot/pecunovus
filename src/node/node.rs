//! Node orchestration: wire up network, consensus, ledger, state, runtime, txpool, rpc.
//!
//! This file performs pragmatic wiring using the module APIs we created earlier.
//! Replace adapter shims if your concrete types/signatures differ.

use anyhow::Result;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{info, error};
use crate::node::service_handle::ServiceHandle;

#[derive(Clone)]
pub struct NodeConfig {
    pub data_dir: String,
    pub bind_addr: String,
    pub rpc_addr: String,
    pub bootstrap_peers: Vec<String>,
    pub max_txpool_size: usize,
}

/// Main Node object
pub struct Node {
    cfg: NodeConfig,
}

impl Node {
    pub fn new(cfg: NodeConfig) -> Self {
        Self { cfg }
    }

    /// Start the node: spawn subsystems and return ServiceHandle for graceful shutdown.
    pub async fn start(self) -> Result<ServiceHandle> {
        // create service handle + rx for tasks to observe shutdown
        let (mut svc_handle, shutdown_rx) = ServiceHandle::new();

        // set up data directories
        let ledger_path = format!("{}/ledger", self.cfg.data_dir);
        std::fs::create_dir_all(&ledger_path)?;

        // -----------------------
        // Ledger
        // -----------------------
        let ledger = crate::ledger::Ledger::new(&ledger_path);

        // -----------------------
        // State (AccountStore -> AccountCache)
        // -----------------------
        let account_store = Arc::new(crate::state::account_db::InMemAccountStore::new())
            as Arc<dyn crate::state::account_db::AccountStore>;
        let account_cache = crate::state::account_cache::AccountCache::new(account_store.clone());

        // -----------------------
        // TxPool
        // -----------------------
        let pool = Arc::new(crate::txpool::pool::TxPool::new(
            self.cfg.max_txpool_size,
            std::time::Duration::from_secs(60 * 60),
            10_000,
        ));

        // -----------------------
        // AccountLocks & Executor
        // -----------------------
        let locks = crate::state::account_lock::AccountLocks::new(256);
        let executor = Arc::new(crate::runtime::executor::Executor::new(account_cache.clone(), locks.clone()));

        // -----------------------
        // Networking (ConnectionManager)
        // -----------------------
        // inbound channel where connection manager will push (SocketAddr, WireMessage)
        let (inbound_tx, mut inbound_rx) = tokio::sync::mpsc::unbounded_channel::<(std::net::SocketAddr, crate::network::message::WireMessage)>();
        let peerstore = crate::network::peerstore::PeerStore::new();

        let conn_manager = Arc::new(crate::network::manager::ConnectionManager::new(
            inbound_tx.clone(),
            peerstore.clone(),
            10_000,
            8,
        ));

        // Start network listener task
        {
            let cm = conn_manager.clone();
            let bind = self.cfg.bind_addr.clone();
            let mut shutdown_rx = shutdown_rx.clone();
            let h: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
                // Start listener
                if let Err(e) = cm.start_listener(&bind).await {
                    error!("ConnectionManager start_listener failed: {:?}", e);
                    return Err(anyhow::anyhow!(e));
                }

                // Observe shutdown to optionally close manager (if you add close API)
                loop {
                    if *shutdown_rx.borrow() {
                        info!("network listener observed shutdown");
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                }
                Ok(())
            });
            svc_handle.attach(h);
        }

        // Connect to bootstrap peers
        for peer in &self.cfg.bootstrap_peers {
            let cm = conn_manager.clone();
            let p = peer.clone();
            tokio::spawn(async move {
                cm.connect_peer(p).await;
            });
        }

        // -----------------------
        // Consensus engine
        // -----------------------
        // Adapter implementing consensus::NetworkSender using ConnectionManager.
        #[derive(Clone)]
        struct NetSenderAdapter {
            cm: Arc<crate::network::manager::ConnectionManager>,
        }

        impl crate::consensus::NetworkSender for NetSenderAdapter {
            fn send_proposal(&self, proposal: crate::consensus::types::BlockProposal) {
                let msg = crate::network::message::NetworkMessage::Consensus(
                    crate::network::message::ConsensusMessage::Proposal(proposal),
                );
                let cm = self.cm.clone();
                tokio::spawn(async move {
                    let _ = cm.broadcast(msg).await;
                });
            }
            fn send_vote(&self, vote: crate::consensus::types::Vote) {
                let msg = crate::network::message::NetworkMessage::Consensus(
                    crate::network::message::ConsensusMessage::Vote(vote),
                );
                let cm = self.cm.clone();
                tokio::spawn(async move {
                    let _ = cm.broadcast(msg).await;
                });
            }
        }

        let net_sender = NetSenderAdapter { cm: conn_manager.clone() };
        let consensus = Arc::new(crate::consensus::ConsensusEngine::new("node-local".into(), 100, Arc::new(net_sender)));

        // -----------------------
        // Inbound dispatcher: route incoming network messages to consensus/txpool/etc.
        // -----------------------
        {
            let consensus = consensus.clone();
            let pool = pool.clone();
            let executor = executor.clone();
            let mut shutdown_rx = shutdown_rx.clone();
            let h: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
                while !*shutdown_rx.borrow() {
                    if let Some((_addr, msg)) = inbound_rx.recv().await {
                        match msg {
                            crate::network::message::NetworkMessage::Consensus(cmsg) => {
                                // forward to consensus engine
                                consensus.handle_message(cmsg).await;
                            }
                            crate::network::message::NetworkMessage::Gossip(gmsg) => {
                                // naive: if gossip contains tx bytes, try to deserialize and insert into pool
                                match gmsg {
                                    crate::network::message::GossipMessage::Transaction(data) => {
                                        if let Ok(tx) = bincode::deserialize::<crate::txpool::pool::Tx>(&data) {
                                            let validator = crate::txpool::ingest::SimpleValidator::new(account_cache.clone());
                                            let ingestor = crate::txpool::ingest::TxIngestor::new(pool.clone(), std::sync::Arc::new(validator));
                                            let _ = ingestor.ingest(tx).await;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            _ => {}
                        }
                    } else {
                        // channel closed
                        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    }
                }
                info!("inbound dispatcher shutting down");
                Ok(())
            });
            svc_handle.attach(h);
        }

        // -----------------------
        // Tx forwarder (background)
        // -----------------------
        {
            // implement NetworkSender for forwarder using ConnectionManager
            #[derive(Clone)]
            struct ForwNetAdapter {
                cm: Arc<crate::network::manager::ConnectionManager>,
            }
            #[async_trait::async_trait]
            impl crate::txpool::forwarder::NetworkSender for ForwNetAdapter {
                async fn send_to_peer(&self, _peer_addr: &str, _topic: &str, _data: Vec<u8>) -> Result<(), anyhow::Error> {
                    // TODO: implement targeted send using ConnectionManager if available.
                    // For now broadcast as fallback.
                    let _ = self.cm.broadcast(crate::network::message::NetworkMessage::Gossip(crate::network::message::GossipMessage::Transaction(_data))).await;
                    Ok(())
                }
                async fn broadcast(&self, _topic: &str, data: Vec<u8>) -> Result<(), anyhow::Error> {
                    let _ = self.cm.broadcast(crate::network::message::NetworkMessage::Gossip(crate::network::message::GossipMessage::Transaction(data))).await;
                    Ok(())
                }
            }

            let net = Arc::new(ForwNetAdapter { cm: conn_manager.clone() });
            let cfg = crate::txpool::forwarder::ForwardConfig {
                leader_addr: None,
                gossip: true,
                batch_size: 64,
                poll_interval_ms: 100,
            };

            let forwarder = crate::txpool::forwarder::TxForwarder::new(pool.clone(), net, cfg, shutdown_rx.clone());
            let h: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
                forwarder.run().await;
                Ok(())
            });
            svc_handle.attach(h);
        }

        // -----------------------
        // RPC server
        // -----------------------
        {
            #[derive(Clone)]
            struct NodeRpcDeps {
                consensus: Arc<crate::consensus::ConsensusEngine<NetSenderAdapter>>,
                ledger: Arc<std::sync::Mutex<crate::ledger::Ledger>>,
                pool: Arc<crate::txpool::pool::TxPool>,
                cache: crate::state::account_cache::AccountCache,
            }

            #[async_trait::async_trait]
            impl crate::rpc::handlers::RpcDeps for NodeRpcDeps {
                async fn consensus_snapshot(&self) -> crate::consensus::consensus_state::ConsensusSnapshot {
                    self.consensus.snapshot().await
                }
                async fn submit_transaction(&self, tx: crate::txpool::pool::Tx) -> Result<crate::txpool::ingest::IngestResult> {
                    let validator = crate::txpool::ingest::SimpleValidator::new(self.cache.clone());
                    let ingestor = crate::txpool::ingest::TxIngestor::new(self.pool.clone(), std::sync::Arc::new(validator));
                    Ok(ingestor.ingest(tx).await?)
                }
                async fn get_block(&self, slot: u64) -> Result<Option<Vec<u8>>> {
                    Ok(self.ledger.lock().unwrap().get_block(slot))
                }
                async fn get_account(&self, key: &str) -> Result<Option<crate::state::account_db::Account>> {
                    Ok(self.cache.get(&key.to_string()).ok().flatten())
                }
                async fn mempool_size(&self) -> usize {
                    self.pool.len()
                }
            }

            let deps = Arc::new(NodeRpcDeps {
                consensus: consensus.clone(),
                ledger: Arc::new(std::sync::Mutex::new(ledger)),
                pool: pool.clone(),
                cache: account_cache.clone(),
            });

            let rpc_addr = self.cfg.rpc_addr.parse()?;
            let auth = crate::rpc::auth::AuthConfig::disabled();
            let server = crate::rpc::server::RpcServer::new(rpc_addr, deps, auth);

            let h: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
                if let Err(e) = server.start().await {
                    error!("RPC server failed: {:?}", e);
                    return Err(anyhow::anyhow!(e));
                }
                Ok(())
            });
            svc_handle.attach(h);
        }

        info!("Node started, RPC: {}, network: {}", self.cfg.rpc_addr, self.cfg.bind_addr);
        Ok(svc_handle)
    }
}
