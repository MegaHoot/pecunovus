//! Network module - transport, peer management, handshake, gossip, connection manager.
//! Exposes NetworkManager, Connection, Message types.

pub mod codec;
pub mod message;
pub mod handshake;
pub mod connection;
pub mod manager;
pub mod gossip;
pub mod peerstore;
pub mod transport;

pub use message::{WireMessage, HandshakeMsg};
pub use connection::Connection;
pub use manager::ConnectionManager;
pub use gossip::Gossiper;
pub use peerstore::PeerStore;
