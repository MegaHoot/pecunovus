use ed25519_dalek::{Keypair as DalekKeypair, PublicKey as DalekPublic, SecretKey};
use rand_core::OsRng;
use serde::{Serialize, Deserialize};
use anyhow::Result;

#[derive(Clone, Serialize, Deserialize)]
pub struct PublicKey(pub [u8; 32]);

#[derive(Clone, Serialize, Deserialize)]
pub struct PrivateKey(pub [u8; 32]);

#[derive(Clone)]
pub struct Keypair {
    pub keypair: DalekKeypair,
}

impl Keypair {
    /// Generate a new random keypair
    pub fn generate() -> Self {
        let kp = DalekKeypair::generate(&mut OsRng);
        Self { keypair: kp }
    }

    /// Construct from raw bytes
    pub fn from_bytes(secret: &[u8]) -> Result<Self> {
        let sk = SecretKey::from_bytes(secret)?;
        let pk = DalekPublic::from(&sk);
        let kp = DalekKeypair { secret: sk, public: pk };
        Ok(Self { keypair: kp })
    }

    /// Get public key
    pub fn public(&self) -> PublicKey {
        PublicKey(self.keypair.public.to_bytes())
    }

    /// Export secret as bytes
    pub fn secret_bytes(&self) -> [u8; 32] {
        self.keypair.secret.to_bytes()
    }
}
