use ed25519_dalek::{Signature as DalekSig, Signer as DalekSigner, Verifier as DalekVerifier};
use serde::{Serialize, Deserialize};
use anyhow::{Result, anyhow};
use crate::crypto::{Keypair, PublicKey};

#[derive(Clone, Serialize, Deserialize)]
pub struct Signature(pub [u8; 64]);

/// Trait for signing
pub trait Signer {
    fn sign(&self, msg: &[u8]) -> Signature;
}

/// Trait for verifying
pub trait Verifier {
    fn verify(&self, msg: &[u8], sig: &Signature) -> Result<()>;
}

impl Signer for Keypair {
    fn sign(&self, msg: &[u8]) -> Signature {
        let sig = self.keypair.sign(msg);
        Signature(sig.to_bytes())
    }
}

impl Verifier for PublicKey {
    fn verify(&self, msg: &[u8], sig: &Signature) -> Result<()> {
        let pk = ed25519_dalek::PublicKey::from_bytes(&self.0)?;
        let ds = DalekSig::from_bytes(&sig.0)?;
        pk.verify(msg, &ds).map_err(|_| anyhow!("signature verification failed"))
    }
}
