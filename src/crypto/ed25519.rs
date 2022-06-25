use super::SignatureScheme;

use ed25519_dalek::Signer;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ed25519;

pub struct PrivateKey(pub ed25519_dalek::Keypair);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PublicKey(pub ed25519_dalek::PublicKey);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Signature(pub ed25519_dalek::Signature);

impl SignatureScheme for Ed25519 {
    type Pub = PublicKey;
    type Priv = PrivateKey;
    type Sig = Signature;
    fn generate_keys(_seed: &[u8]) -> (PublicKey, PrivateKey) {
        let mut csprng = OsRng {};
        let keypair = ed25519_dalek::Keypair::generate(&mut csprng);
        (PublicKey(keypair.public), PrivateKey(keypair))
    }
    fn sign(sk: &PrivateKey, message: &[u8]) -> Signature {
        Signature(sk.0.sign(message))
    }
    fn verify(pk: &PublicKey, message: &[u8], sig: &Signature) -> bool {
        unimplemented!();
    }
}

impl std::fmt::Display for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        unimplemented!();
    }
}

#[derive(Error, Debug)]
pub enum ParsePublicKeyError {
    #[error("public key invalid")]
    Invalid,
}

impl FromStr for PublicKey {
    type Err = ParsePublicKeyError;
    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        unimplemented!();
    }
}
