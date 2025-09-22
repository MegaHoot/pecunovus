use crate::network::message::HandshakeMsg;
use ed25519_dalek::{Keypair, PublicKey, Signature, Signer, Verifier};
use rand::rngs::OsRng;
use rand::RngCore;

/// Create a signed handshake message from Keypair
pub fn create_handshake(kp: &Keypair, protocol_version: u16, features: Vec<String>) -> HandshakeMsg {
    // generate 16-byte nonce
    let mut nonce = [0u8; 16];
    OsRng.fill_bytes(&mut nonce);

    let node_id = hex::encode(kp.public.to_bytes());

    // construct bytes to sign: node_id || proto_be || nonce
    let mut to_sign = Vec::with_capacity(node_id.len() + 2 + nonce.len());
    to_sign.extend_from_slice(node_id.as_bytes());
    to_sign.extend_from_slice(&protocol_version.to_be_bytes());
    to_sign.extend_from_slice(&nonce);

    let sig = kp.sign(&to_sign).to_bytes().to_vec();

    HandshakeMsg {
        node_id,
        protocol_version,
        features,
        signature: sig,
        nonce: nonce.to_vec(),
    }
}

/// verify a handshake (returns Ok(()) if signature valid)
pub fn verify_handshake(hs: &HandshakeMsg) -> Result<(), &'static str> {
    let pk_bytes = hex::decode(&hs.node_id).map_err(|_| "invalid node_id hex")?;
    let pk = PublicKey::from_bytes(&pk_bytes).map_err(|_| "invalid public key")?;

    let mut to_verify = Vec::with_capacity(hs.node_id.len() + 2 + hs.nonce.len());
    to_verify.extend_from_slice(hs.node_id.as_bytes());
    to_verify.extend_from_slice(&hs.protocol_version.to_be_bytes());
    to_verify.extend_from_slice(&hs.nonce);

    let signature = Signature::from_bytes(&hs.signature).map_err(|_| "invalid signature bytes")?;
    pk.verify(&to_verify, &signature).map_err(|_| "signature verify failed")
}
