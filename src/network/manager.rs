use crate::network::connection::{Connection, InboundSender};
use crate::network::message::{WireMessage, HandshakeMsg};
use crate::network::handshake;
use crate::network::peerstore::PeerStore;
use crate::network::gossip::Gossiper;
use crate::network::transport;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{sleep, Duration};
use tracing::{info, warn};
use ed25519_dalek::Keypair;
use rand::rngs::OsRng;
use anyhow::Result;

/// Manager holds active connections and coordinates reconnect/backoff and inbound dispatch.
pub struct ConnectionManager {
    /// active connections map: addr -> Connection handle
    pub conns: Arc<DashMap<String, ConnectionHandle>>,
    inbound_tx: InboundSender,
    pub peerstore: PeerStore,
    pub gossiper: Gossiper,
    local_kp: Keypair,
    protocol_version: u16,
}

pub struct ConnectionHandle {
    pub outbound: tokio::sync::mpsc::Sender<WireMessage>,
    shutdown: oneshot::Sender<()>,
}

impl ConnectionHandle {
    pub fn new(outbound: tokio::sync::mpsc::Sender<WireMessage>, shutdown: oneshot::Sender<()>) -> Self {
        Self { outbound, shutdown }
    }

    pub async fn send(&self, msg: WireMessage) -> Result<()> {
        self.outbound.send(msg).await.map_err(|_| anyhow::anyhow!("send failed"))
    }

    pub fn close(self) {
        let _ = self.shutdown.send(());
    }
}

impl ConnectionManager {
    pub fn new(inbound_tx: InboundSender, peerstore: PeerStore, dedup_cap: usize, fanout: usize) -> Self {
        let mut rng = OsRng{};
        let kp = Keypair::generate(&mut rng);
        let goss = Gossiper::new(peerstore.clone(), dedup_cap, fanout);
        Self {
            conns: Arc::new(DashMap::new()),
            inbound_tx,
            peerstore,
            gossiper: goss,
            local_kp: kp,
            protocol_version: 1,
        }
    }

    /// Start listener to accept incoming connections and spawn Connection tasks.
    pub async fn start_listener(&self, bind_addr: &str) -> Result<()> {
        let listener = transport::bind(bind_addr).await?;
        info!("listening on {}", bind_addr);
        let inbound = self.inbound_tx.clone();
        let conns_map = self.conns.clone();
        let peerstore = self.peerstore.clone();
        let local_kp = self.local_kp.clone();
        let protocol_version = self.protocol_version;

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, peer_addr)) => {
                        let peer_addr_s = peer_addr.to_string();
                        let inbound_clone = inbound.clone();
                        let conns_map = conns_map.clone();
                        let peerstore = peerstore.clone();
                        let local_kp = local_kp.clone();
                        tokio::spawn(async move {
                            info!("accepted connection from {}", peer_addr_s);
                            match Connection::spawn(stream, inbound_clone).await {
                                Ok(conn) => {
                                    // immediate handshake exchange: read first handshake on read loop; but we may also perform our handshake here.
                                    // For simplicity we'll broadcast our handshake by sending outbound after small delay.
                                    let (shutdown_tx, _) = oneshot::channel::<()>();
                                    // keep storing outbound so other parts can send
                                    // Note: Connection::spawn returned with its own outbound channel, but we don't have that exposed here.
                                    // Instead, update conns_map with a placeholder until we upgrade (in production we should return Connection struct outward properly).
                                    // For now: record peer in peerstore
                                    peerstore.add_peer(peer_addr_s.clone(), peer_addr_s.clone()).await;
                                    info!("peerstore updated with {}", peer_addr_s);
                                }
                                Err(e) => {
                                    warn!("connection spawn failed for {}: {:?}", peer_addr_s, e);
                                }
                            }
                        });
                    }
                    Err(e) => {
                        warn!("accept failed: {:?}", e);
                        sleep(Duration::from_millis(100)).await;
                    }
                }
            }
        });
        Ok(())
    }

    /// Connect to a peer with reconnection/backoff and store active connection handle.
    pub async fn connect_peer(&self, addr: String) {
        let inbound = self.inbound_tx.clone();
        let conns_map = self.conns.clone();
        let peerstore = self.peerstore.clone();
        let local_kp = self.local_kp.clone();
        let protocol_version = self.protocol_version;

        tokio::spawn(async move {
            let mut backoff = 500u64; // ms
            loop {
                match transport::connect(&addr).await {
                    Ok(stream) => {
                        match Connection::spawn(stream, inbound.clone()).await {
                            Ok(conn) => {
                                info!("connected to peer {}", addr);
                                // create outbound handle info
                                // Here we need to extract outbound sender; but Connection::spawn returns Connection with outbound sender inside.
                                // To get it, rework Connection::spawn to return Connection struct with outbound exposed (we did).
                                // For now we store a placeholder; in your integration change Connection::spawn signature to return outbound & shutdown.
                                // Add to peerstore
                                peerstore.add_peer(addr.clone(), addr.clone()).await;
                                // break reconnection loop for now
                                break;
                            }
                            Err(e) => {
                                warn!("spawn failed for {}: {:?}", addr, e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("connect failed to {}: {:?}", addr, e);
                    }
                }
                sleep(Duration::from_millis(backoff)).await;
                backoff = (backoff * 2).min(30_000);
            }
        });
    }

    /// Broadcast a WireMessage to all active connections (best-effort)
    pub async fn broadcast(&self, msg: WireMessage) {
        // send to each connection handle in map
        for entry in self.conns.iter() {
            if let Some(handle) = entry.value().outbound.clone().try_reserve() {
                // We can't use try_reserve on Sender; easier: clone sender and send
            }
            let tx = entry.value().outbound.clone();
            let msg_clone = msg.clone();
            let _ = tokio::spawn(async move {
                let _ = tx.send(msg_clone).await;
            });
        }
    }
}
