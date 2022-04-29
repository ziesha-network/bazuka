use ff::{Field, PrimeField};
use serde::{Deserialize, Serialize};
use zeekit::{eddsa, mimc, Fr};

use super::SignatureScheme;

use std::str::FromStr;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EdDSA;

#[derive(Error, Debug)]
pub enum ParsePublicKeyError {
    #[error("public key invalid")]
    Invalid,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EdDSAPublicKey(pub eddsa::PublicKey);

impl std::fmt::Display for EdDSAPublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "0x{}", if self.0 .0 .1 { 3 } else { 2 })?;
        for byte in self.0 .0 .0.to_repr().as_ref().iter().rev() {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

impl FromStr for EdDSAPublicKey {
    type Err = ParsePublicKeyError;
    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        if s.len() != 67 {
            return Err(ParsePublicKeyError::Invalid);
        }
        let oddity = if s.starts_with("0x3") {
            true
        } else if s.starts_with("0x2") {
            false
        } else {
            return Err(ParsePublicKeyError::Invalid);
        };
        s = &s[3..];
        let bytes = (0..32)
            .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16))
            .rev()
            .collect::<Result<Vec<u8>, std::num::ParseIntError>>()
            .map_err(|_| ParsePublicKeyError::Invalid)?;
        let mut repr = Fr::zero().to_repr();
        repr.as_mut().clone_from_slice(&bytes);
        Ok(EdDSAPublicKey(eddsa::PublicKey(eddsa::PointCompressed(
            Fr::from_repr(repr).unwrap(),
            oddity,
        ))))
    }
}

#[derive(Clone)]
pub struct PrivateKey(eddsa::PrivateKey);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Signature(eddsa::Signature);

fn mimc_u8(inp: &[u8]) -> Fr {
    mimc::mimc(inp.iter().map(|u| Fr::from(*u as u64)).collect())
}

impl SignatureScheme for EdDSA {
    type Pub = EdDSAPublicKey;
    type Priv = PrivateKey;
    type Sig = Signature;
    fn generate_keys(seed: &[u8]) -> (EdDSAPublicKey, PrivateKey) {
        let randomness = mimc_u8(seed);
        let scalar = mimc::mimc(vec![randomness]);
        let (pk, sk) = eddsa::generate_keys(randomness, scalar);
        (EdDSAPublicKey(pk), PrivateKey(sk))
    }
    fn sign(sk: &PrivateKey, message: &[u8]) -> Signature {
        let hash = mimc::mimc(message.iter().map(|u| Fr::from(*u as u64)).collect());
        Signature(eddsa::sign(&sk.0, hash))
    }
    fn verify(pk: &EdDSAPublicKey, message: &[u8], sig: &Signature) -> bool {
        let hash = mimc::mimc(message.iter().map(|u| Fr::from(*u as u64)).collect());
        eddsa::verify(&pk.0, hash, &sig.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zeekit::eddsa::BASE;
    use zeekit::Fr;

    #[test]
    fn test_public_key_compression() {
        let p1 = BASE.multiply(&Fr::from(123 as u64));
        let p2 = p1.compress().decompress();

        assert_eq!(p1, p2);
    }

    #[test]
    fn test_signature_verification() {
        let (pk, sk) = EdDSA::generate_keys(b"ABC");
        let msg = b"Hi this a transaction!";
        let fake_msg = b"Hi this a fake transaction!";
        let sig = EdDSA::sign(&sk, msg);

        assert!(EdDSA::verify(&pk, msg, &sig));
        assert!(!EdDSA::verify(&pk, fake_msg, &sig));
    }
}
