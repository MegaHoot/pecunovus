//! Crypto module: key management, signing, verification, VRF.
//!
//! - Keys: generation, storage, serialization
//! - Sign: Ed25519 signatures
//! - VRF: verifiable randomness for leader election

pub mod keys;
pub mod sign;
pub mod vrf;

pub use keys::{Keypair, PublicKey, PrivateKey};
pub use sign::{Signature, Signer, Verifier};
pub use vrf::{VrfKeypair, VrfProof};
