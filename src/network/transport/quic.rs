// src/network/transport/quic.rs
use anyhow::Result;
use quinn::{Endpoint, ServerConfig, CertificateChain, PrivateKey, ClientConfig, Certificate};
use rcgen::generate_simple_self_signed;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use bytes::{BytesMut, BufMut};
use tokio::sync::mpsc;
use tracing::{info, warn};
use std::time::Duration;

/// Type for inbound frames: (peer_addr, raw_frame_bytes)
pub type InboundFrame = (SocketAddr, Vec<u8>);
pub type InboundSender = mpsc::UnboundedSender<InboundFrame>;

/// Result returned by connect_to_peer: a handle you can use to send frames to peer.
pub struct QuicHandle {
    // send side of the established bi-stream
    send: quinn::SendStream,
    // optionally keep remote socket addr for logging
    pub peer_addr: SocketAddr,
}

impl QuicHandle {
    /// Send a single length-prefixed frame (u32 BE length + bytes)
    pub async fn send_frame(&mut self, frame: &[u8]) -> Result<()> {
        // write 4-byte length prefix
        let len = (frame.len() as u32).to_be_bytes();
        self.send.write_all(&len).await?;
        self.send.write_all(frame).await?;
        self.send.flush().await?;
        Ok(())
    }

    /// Close the sending stream gracefully.
    pub async fn close(mut self) -> Result<()> {
        self.send.finish().await?;
        Ok(())
    }
}

/// Helper to create a self-signed server config (development).
/// Returns (ServerConfig, cert_der_bytes)
pub fn make_server_config_self_signed() -> Result<(ServerConfig, Vec<u8>)> {
    // generate cert for localhost (or use subject alt names)
    let cert = generate_simple_self_signed(vec!["localhost".into()])?;
    let cert_pem = cert.serialize_pem()?;
    let key_pem = cert.serialize_private_key_pem();

    // convert to rustls types via quinn helpers
    let cert_der = pem_to_der(&cert_pem)?;
    let key_der = pem_to_der(&key_pem)?;

    // build server config
    let cert_chain = CertificateChain::from_certs(vec![Certificate::from_der(&cert_der)?]);
    let priv_key = PrivateKey::from_der(&key_der)?;
    let mut server_config = ServerConfig::with_single_cert(cert_chain, priv_key)?;
    // tune parameters for performance — these are sensible defaults, tune further in prod
    let mut transport_config = quinn::TransportConfig::default();
    transport_config.keep_alive_interval(Some(Duration::from_secs(10)));
    server_config.transport = Arc::new(transport_config);

    Ok((server_config, cert_der))
}

fn pem_to_der(pem: &str) -> Result<Vec<u8>> {
    // naive extraction of base64 between PEM boundaries
    let (start, end) = if pem.contains("BEGIN CERTIFICATE") {
        ("-----BEGIN CERTIFICATE-----", "-----END CERTIFICATE-----")
    } else if pem.contains("BEGIN PRIVATE KEY") {
        ("-----BEGIN PRIVATE KEY-----", "-----END PRIVATE KEY-----")
    } else {
        return Err(anyhow::anyhow!("unsupported pem"));
    };
    let body = pem.split(start).nth(1).ok_or_else(|| anyhow::anyhow!("pem missing start"))?
        .split(end).next().ok_or_else(|| anyhow::anyhow!("pem missing end"))?;
    let body = body.replace("\r", "").replace("\n", "");
    let der = base64::decode(body)?;
    Ok(der)
}

/// Create a server QUIC endpoint bound to bind_addr; returns (Endpoint, server_cert_der)
/// You should pass the `server_cert_der` to bootstrap peers so they can construct ClientConfig.
pub async fn make_server_endpoint(bind_addr: &str) -> Result<(Endpoint, Vec<u8>)> {
    let (server_config, cert_der) = make_server_config_self_signed()?;

    // bind UDP socket and create endpoint
    let mut endpoint = Endpoint::server(server_config, bind_addr.parse()?)?;
    Ok((endpoint, cert_der))
}

/// Accept incoming QUIC connections and spawn a handler for each.
/// - `endpoint` is a quinn::Endpoint returned by `make_server_endpoint`
/// - `inbound_tx` will receive raw frames: (peer_addr, frame_bytes)
/// This function returns immediately after spawning an accept-loop task.
pub async fn accept_incoming(endpoint: &Endpoint, inbound_tx: InboundSender) -> Result<()> {
    let mut incoming = endpoint.incoming();
    // spawn background acceptor
    tokio::spawn(async move {
        while let Some(conn) = incoming.next().await {
            match conn.await {
                Ok(connection) => {
                    let remote_addr = connection.remote_address();
                    info!("QUIC incoming connection from {}", remote_addr);
                    // spawn handler for streams on this connection
                    let inbound = inbound_tx.clone();
                    tokio::spawn(async move {
                        // accept bi-directional streams
                        loop {
                            match connection.accept_bi().await {
                                Ok((mut send, mut recv)) => {
                                    let peer_sock = remote_addr.as_std().to_owned();
                                    let inbound_clone = inbound.clone();
                                    // spawn reading task for this stream
                                    tokio::spawn(async move {
                                        if let Err(e) = read_loop_quic_stream(&mut recv, &peer_sock, inbound_clone).await {
                                            warn!("error reading quic bi stream: {:?}", e);
                                        }
                                    });
                                    // you can keep send stream for outbound messages per connection if desired
                                }
                                Err(e) => {
                                    // no stream available or connection closed
                                    warn!("accept_bi error: {:?}", e);
                                    break;
                                }
                            }
                        }
                    });
                }
                Err(e) => {
                    warn!("incoming connection failed: {:?}", e);
                }
            }
        }
    });

    Ok(())
}

/// Connect to a remote QUIC server (addr) using server_cert for validation.
/// Returns a QuicHandle (with open bi-stream send side) and also spawns a reader that forwards inbound frames to inbound_tx.
pub async fn connect_to_peer(addr: &str, server_cert_der: &[u8], inbound_tx: InboundSender) -> Result<QuicHandle> {
    // Build client config trusting server_cert_der
    let cert = Certificate::from_der(server_cert_der)?;
    let mut roots = rustls::RootCertStore::empty();
    roots.add(&rustls::Certificate(server_cert_der.to_vec())).map_err(|e| anyhow::anyhow!(format!("root add error {:?}", e)))?;
    let mut client_crypto = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(roots)
        .with_no_client_auth();
    // set client transport config if needed
    let mut client_config = ClientConfig::default();
    client_config.crypto = Arc::new(client_crypto);

    let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
    endpoint.set_default_client_config(client_config);

    let connecting = endpoint.connect(addr.parse()?, "localhost")?; // server name must match cert SAN; use "localhost" for dev
    let connection = connecting.await?;
    let peer_addr = connection.remote_address().as_std().to_owned();

    // open a bi-directional stream to the server
    let (mut send, mut recv) = connection.open_bi().await?;
    // spawn reader loop to forward inbound frames
    let inbound_clone = inbound_tx.clone();
    tokio::spawn(async move {
        if let Err(e) = read_loop_quic_stream(&mut recv, &peer_addr, inbound_clone).await {
            warn!("quic read loop ended for {}: {:?}", peer_addr, e);
        }
    });

    Ok(QuicHandle { send, peer_addr })
}

/// Read loop for a QUIC RecvStream: read length-prefixed frames and forward to inbound channel.
async fn read_loop_quic_stream(recv: &mut quinn::RecvStream, peer_addr: &std::net::SocketAddr, inbound_tx: InboundSender) -> Result<()> {
    loop {
        // read 4-byte length prefix
        let mut len_buf = [0u8; 4];
        if let Err(e) = recv.read_exact(&mut len_buf).await {
            // connection/stream closed
            return Err(anyhow::anyhow!(format!("read_exact failed: {:?}", e)));
        }
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        recv.read_exact(&mut buf).await?;
        let _ = inbound_tx.send((peer_addr.clone(), buf));
    }
}
