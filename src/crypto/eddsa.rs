use std::ops::*;

use ff::{Field, PrimeField};
use num_bigint::BigUint;
use num_integer::Integer;
use serde::{Deserialize, Serialize};

use super::curve::*;
use super::field::Fr;
use super::SignatureScheme;
use crate::core::number::U256;

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
pub struct EdDSAPublicKey(pub PointCompressed);

impl std::fmt::Display for EdDSAPublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "0x{}", if self.0 .1 { 3 } else { 2 })?;
        for byte in self.0 .0.to_repr().as_ref().iter().rev() {
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
        Ok(EdDSAPublicKey(PointCompressed(
            Fr::from_repr(repr).unwrap(),
            oddity,
        )))
    }
}

#[derive(Clone)]
pub struct PrivateKey {
    pub public_key: PointAffine,
    pub randomness: U256,
    pub scalar: U256,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Signature {
    pub r: PointAffine,
    pub s: U256,
}

impl SignatureScheme for EdDSA {
    type Pub = EdDSAPublicKey;
    type Priv = PrivateKey;
    type Sig = Signature;
    fn generate_keys(seed: &[u8]) -> (EdDSAPublicKey, PrivateKey) {
        let (randomness, scalar) = U256::generate_two(seed);
        let point = BASE.multiply(&scalar);
        let pk = EdDSAPublicKey(point.compress());
        (
            pk.clone(),
            PrivateKey {
                public_key: point,
                randomness,
                scalar,
            },
        )
    }
    fn sign(sk: &PrivateKey, message: &[u8]) -> Signature {
        // r=H(b,M)
        let mut randomized_message = sk.randomness.to_bytes().to_vec();
        randomized_message.extend(message);
        let (r, _) = U256::generate_two(&randomized_message);

        // R=rB
        let rr = BASE.multiply(&r);

        // h=H(R,A,M)
        let mut inp = Vec::new();
        inp.extend_from_slice(rr.0.to_repr().as_ref());
        inp.extend_from_slice(rr.1.to_repr().as_ref());
        inp.extend_from_slice(sk.public_key.0.to_repr().as_ref());
        inp.extend_from_slice(sk.public_key.1.to_repr().as_ref());
        inp.extend(message);
        let (h, _) = U256::generate_two(&inp);

        // s = (r + ha) mod ORDER
        let mut s = BigUint::from_bytes_le(&r.0);
        let mut ha = BigUint::from_bytes_le(&h.0);
        ha.mul_assign(&BigUint::from_bytes_le(&sk.scalar.0));
        s.add_assign(&ha);
        s = s.mod_floor(&*ORDER);

        Signature {
            r: rr,
            s: U256::from_le_bytes(&s.to_bytes_le()),
        }
    }
    fn verify(pk: &EdDSAPublicKey, message: &[u8], sig: &Signature) -> bool {
        let pk = pk.0.decompress();

        // h=H(R,A,M)
        let mut inp = Vec::new();
        inp.extend_from_slice(sig.r.0.to_repr().as_ref());
        inp.extend_from_slice(sig.r.1.to_repr().as_ref());
        inp.extend_from_slice(pk.0.to_repr().as_ref());
        inp.extend_from_slice(pk.1.to_repr().as_ref());
        inp.extend(message);
        let (h, _) = U256::generate_two(&inp);

        let sb = BASE.multiply(&sig.s);

        let mut r_plus_ha = pk.multiply(&h);
        r_plus_ha.add_assign(&sig.r);

        r_plus_ha == sb
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_key_compression() {
        let scalar = U256::generate(b"hi");
        let p1 = BASE.multiply(&scalar);

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
