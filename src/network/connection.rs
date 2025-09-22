use crate::network::codec::FrameCodec;
use crate::network::message::WireMessage;
use bincode;
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};
use tokio_util::codec::Framed;
use tracing::{info, warn};
use anyhow::Result;

/// Outbound channel capacity per connection
pub const OUT_CAP: usize = 1024;

/// Sender used by ConnectionManager to receive inbound wire messages
pub type InboundSender = mpsc::UnboundedSender<(SocketAddr, WireMessage)>;
/// Outbound sender into a connection
pub type OutboundSender = mpsc::Sender<WireMessage>;

/// A running connection to a peer.
/// It holds an outbound sender; read / write tasks run in background.
pub struct Connection {
    pub peer_addr: SocketAddr,
    pub outbound: OutboundSender,
    shutdown: oneshot::Sender<()>,
}

impl Connection {
    /// Spawn read/write tasks on the supplied TcpStream and return Connection object.
    /// - `inbound_tx` is where deserialized inbound WireMessages will be sent.
    /// - returns Connection with outbound channel you can use to send WireMessage to peer.
    pub async fn spawn(stream: TcpStream, inbound_tx: InboundSender) -> Result<Connection> {
        let peer_addr = stream.peer_addr()?;
        let framed = Framed::new(stream, FrameCodec::new());
        let (mut writer, mut reader) = framed.split();

        // outbound queue
        let (out_tx, mut out_rx) = mpsc::channel::<WireMessage>(OUT_CAP);
        // shutdown signal
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let mut shutdown_rx_read = shutdown_rx;

        // Read loop
        let inbound = inbound_tx.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    biased;
                    _ = &mut shutdown_rx_read => {
                        info!("reader shutting down for {}", peer_addr);
                        return;
                    }
                    maybe = reader.next() => {
                        match maybe {
                            Some(Ok(bytes)) => {
                                // Deserialize
                                match bincode::deserialize::<WireMessage>(&bytes) {
                                    Ok(msg) => {
                                        let _ = inbound.send((peer_addr, msg));
                                    }
                                    Err(e) => {
                                        warn!("bincode deserialize error from {}: {:?}", peer_addr, e);
                                    }
                                }
                            }
                            Some(Err(e)) => {
                                warn!("read error from {}: {:?}", peer_addr, e);
                                return;
                            }
                            None => {
                                info!("peer {} closed connection", peer_addr);
                                return;
                            }
                        }
                    }
                }
            }
        });

        // Write loop
        let mut shutdown_rx_write = shutdown_rx;
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    biased;
                    _ = &mut shutdown_rx_write => {
                        info!("writer shutting down for {}", peer_addr);
                        return;
                    }
                    maybe = out_rx.recv() => {
                        match maybe {
                            Some(msg) => {
                                match bincode::serialize(&msg) {
                                    Ok(bin) => {
                                        if writer.send(Bytes::from(bin)).await.is_err() {
                                            warn!("failed send to {}", peer_addr);
                                            return;
                                        }
                                    }
                                    Err(e) => {
                                        warn!("serialize error sending to {}: {:?}", peer_addr, e);
                                    }
                                }
                            }
                            None => {
                                info!("outbound channel closed for {}", peer_addr);
                                return;
                            }
                        }
                    }
                }
            }
        });

        Ok(Self {
            peer_addr,
            outbound: out_tx,
            shutdown: shutdown_tx,
        })
    }

    /// Send a message to peer via outbound channel (awaits if channel full).
    pub async fn send(&self, msg: WireMessage) -> Result<()> {
        // backpressure: await send
        self.outbound.send(msg).await.map_err(|_| anyhow::anyhow!("send failed"))
    }

    /// Force-close connection (signal read/write tasks to stop)
    pub fn close(self) {
        let _ = self.shutdown.send(());
    }
}
