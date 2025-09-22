use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
use rand_core::OsRng;
use sha2::{Sha512, Digest};
use serde::{Serialize, Deserialize};

#[derive(Clone)]
pub struct VrfKeypair {
    sk: Scalar,
    pk: RistrettoPoint,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct VrfProof {
    pub output: [u8; 32],   // VRF hash output
    pub proof: [u8; 32],    // Simplified proof (not full ZKP yet)
}

impl VrfKeypair {
    pub fn generate() -> Self {
        let sk = Scalar::random(&mut OsRng);
        let pk = sk * RISTRETTO_BASEPOINT_POINT;
        Self { sk, pk }
    }

    pub fn public(&self) -> RistrettoPoint {
        self.pk
    }

    /// Evaluate VRF for input and return (output, proof)
    pub fn evaluate(&self, input: &[u8]) -> VrfProof {
        let h = Sha512::digest(input);
        let x = Scalar::from_hash(Sha512::new_with_prefix(&h));
        let y = self.sk * x * RISTRETTO_BASEPOINT_POINT;

        let out_bytes = y.compress().to_bytes();
        let mut out32 = [0u8; 32];
        out32.copy_from_slice(&out_bytes[..32]);

        VrfProof {
            output: out32,
            proof: self.sk.to_bytes(),
        }
    }

    /// Verify VRF proof
    pub fn verify(&self, input: &[u8], proof: &VrfProof) -> bool {
        let h = Sha512::digest(input);
        let x = Scalar::from_hash(Sha512::new_with_prefix(&h));
        let y = Scalar::from_bytes_mod_order(proof.proof) * x * RISTRETTO_BASEPOINT_POINT;

        let out_bytes = y.compress().to_bytes();
        &out_bytes[..32] == &proof.output
    }
}
