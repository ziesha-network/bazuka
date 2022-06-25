use super::SignatureScheme;

use ed25519_dalek::{Keypair, PublicKey, Signature, Signer};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ed25519;

impl SignatureScheme for Ed25519 {
    type Pub = PublicKey;
    type Priv = Keypair;
    type Sig = Signature;
    fn generate_keys(_seed: &[u8]) -> (PublicKey, Keypair) {
        let mut csprng = OsRng {};
        let keypair: Keypair = Keypair::generate(&mut csprng);
    }
    fn sign(sk: &Keypair, message: &[u8]) -> Signature {
        sk.sign(message)
    }
    fn verify(pk: &PublicKey, message: &[u8], sig: &Signature) -> bool {
        unimplemented!();
    }
}
