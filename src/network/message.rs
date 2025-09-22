use serde::{Serialize, Deserialize};

/// Wire-level messages. Keep stable and small (use prost later if you want cross-language).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum WireMessage {
    Handshake(HandshakeMsg),
    /// Consensus messages should be carried under application-specific payloads.
    /// Use the bytes payload to put serialized ConsensusMessage / Transaction / Block.
    Payload { topic: String, data: Vec<u8> },

    Ping,
    Pong,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HandshakeMsg {
    /// hex-encoded public key (ed25519)
    pub node_id: String,
    /// protocol version
    pub protocol_version: u16,
    /// features/capabilities
    pub features: Vec<String>,
    /// signature over (node_id || protocol_version || nonce)
    pub signature: Vec<u8>,
    /// random nonce
    pub nonce: Vec<u8>,
}
