use super::SignatureScheme;

use crate::core::hash::Hash;
use ed25519_dalek::{Signer, Verifier};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash, Default)]
pub struct Ed25519<H: Hash>(std::marker::PhantomData<H>);

pub struct PrivateKey(pub ed25519_dalek::Keypair);

// Why not derivable?
impl Clone for PrivateKey {
    fn clone(&self) -> Self {
        PrivateKey(ed25519_dalek::Keypair::from_bytes(&self.0.to_bytes()).unwrap())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PublicKey(pub ed25519_dalek::PublicKey);
impl PublicKey {
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes()
    }
}
impl PartialEq<PublicKey> for PublicKey {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bytes() == other.0.to_bytes()
    }
}
impl Eq for PublicKey {}
impl std::hash::Hash for PublicKey {
    fn hash<Hasher>(&self, state: &mut Hasher)
    where
        Hasher: std::hash::Hasher,
    {
        state.write(&self.0.to_bytes());
        state.finish();
    }
}

impl From<PrivateKey> for PublicKey {
    fn from(priv_key: PrivateKey) -> Self {
        Self(priv_key.0.public)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature(pub ed25519_dalek::Signature);

impl Default for Signature {
    fn default() -> Self {
        Self(ed25519_dalek::Signature::from_bytes(&[0u8; 64]).unwrap())
    }
}

impl std::fmt::Display for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0.to_bytes()))
    }
}

impl<H: Hash> SignatureScheme for Ed25519<H> {
    type PubParseError = ParsePublicKeyError;
    type Pub = PublicKey;
    type Priv = PrivateKey;
    type Sig = Signature;
    fn generate_keys(seed: &[u8]) -> (PublicKey, PrivateKey) {
        let mut x = H::hash(seed);
        x.as_mut()[31] &= 0x7f;
        let secret = ed25519_dalek::SecretKey::from_bytes(x.as_ref()).unwrap();
        let public = ed25519_dalek::PublicKey::from(&secret);
        let keypair = ed25519_dalek::Keypair { public, secret };
        (PublicKey(public), PrivateKey(keypair))
    }
    fn sign(sk: &PrivateKey, message: &[u8]) -> Signature {
        Signature(sk.0.sign(message))
    }
    fn verify(pk: &PublicKey, message: &[u8], sig: &Signature) -> bool {
        pk.0.verify(message, &sig.0).is_ok()
    }
}

impl std::fmt::Display for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "ed")?;
        for byte in self.0.as_bytes().iter().rev() {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
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
        if s.len() != 66 || !s.to_lowercase().starts_with("ed") {
            return Err(ParsePublicKeyError::Invalid);
        }
        s = &s[2..];
        let bytes = hex::decode(s)
            .map_err(|_| ParsePublicKeyError::Invalid)?
            .into_iter()
            .rev()
            .collect::<Vec<_>>();
        Ok(PublicKey(
            ed25519_dalek::PublicKey::from_bytes(&bytes)
                .map_err(|_| ParsePublicKeyError::Invalid)?,
        ))
    }
}

impl Default for PublicKey {
    fn default() -> Self {
        Self(ed25519_dalek::PublicKey::from_bytes(&[0u8; 32]).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ed25519_signature_verification() {
        let (pk, sk) = Ed25519::<crate::core::Hasher>::generate_keys(b"ABC");
        let msg = b"salam1";
        let fake_msg = b"salam2";
        let sig = Ed25519::<crate::core::Hasher>::sign(&sk, msg);

        assert!(Ed25519::<crate::core::Hasher>::verify(&pk, msg, &sig));
        assert!(!Ed25519::<crate::core::Hasher>::verify(&pk, fake_msg, &sig));
    }
}
