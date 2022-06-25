use crate::core::hash::{Hash, Sha3Hasher};
use crate::zk::{ZkHasher, ZkScalar, ZkScalarRepr};
use ff::{Field, PrimeField};
use num_bigint::BigUint;
use num_integer::Integer;
use serde::{Deserialize, Serialize};
use std::ops::{AddAssign, MulAssign};

use super::SignatureScheme;

use std::str::FromStr;
use thiserror::Error;

mod curve;
pub use curve::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EdDSA;

#[derive(Error, Debug)]
pub enum ParsePublicKeyError {
    #[error("public key invalid")]
    Invalid,
}

#[derive(Clone)]
pub struct PrivateKey {
    pub public_key: PointAffine,
    pub randomness: ZkScalar,
    pub scalar: ZkScalar,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct PublicKey(pub PointCompressed);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Signature {
    pub r: PointAffine,
    pub s: ZkScalar,
}

pub fn generate_keys(randomness: ZkScalar, scalar: ZkScalar) -> (PublicKey, PrivateKey) {
    let point = BASE.multiply(&scalar);
    let pk = PublicKey(point.compress());
    (
        pk.clone(),
        PrivateKey {
            public_key: point,
            randomness,
            scalar,
        },
    )
}

pub fn sign<H: ZkHasher>(sk: &PrivateKey, message: ZkScalar) -> Signature {
    // r=H(b,M)
    let r = H::hash(&[sk.randomness, message]);

    // R=rB
    let rr = BASE.multiply(&r);

    // h=H(R,A,M)
    let h = H::hash(&[rr.0, rr.1, sk.public_key.0, sk.public_key.1, message]);

    // s = (r + ha) mod ORDER
    let mut s = BigUint::from_bytes_le(r.to_repr().as_ref());
    let mut ha = BigUint::from_bytes_le(h.to_repr().as_ref());
    ha.mul_assign(&BigUint::from_bytes_le(sk.scalar.to_repr().as_ref()));
    s.add_assign(&ha);
    s = s.mod_floor(&*ORDER);
    let s_as_fr = {
        let s_bytes = s.to_bytes_le();
        let mut s_repr = ZkScalarRepr([0u8; 32]);
        s_repr.0[0..s_bytes.len()].copy_from_slice(&s_bytes);
        ZkScalar::from_repr(s_repr).unwrap()
    };

    Signature { r: rr, s: s_as_fr }
}

pub fn verify<H: ZkHasher>(pk: &PublicKey, message: ZkScalar, sig: &Signature) -> bool {
    let pk = pk.0.decompress();

    // h=H(R,A,M)
    let h = H::hash(&[sig.r.0, sig.r.1, pk.0, pk.1, message]);

    let sb = BASE.multiply(&sig.s);

    let mut r_plus_ha = pk.multiply(&h);
    r_plus_ha.add_assign(&sig.r);

    r_plus_ha == sb
}

impl std::fmt::Display for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "0x{}", if self.0 .1 { 3 } else { 2 })?;
        for byte in self.0 .0.to_repr().as_ref().iter().rev() {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

impl FromStr for PublicKey {
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
        let mut repr = ZkScalar::zero().to_repr();
        repr.as_mut().clone_from_slice(&bytes);
        Ok(PublicKey(PointCompressed(
            ZkScalar::from_repr(repr).unwrap(),
            oddity,
        )))
    }
}

fn hash_to_fr(inp: &[u8]) -> ZkScalar {
    let hash = Sha3Hasher::hash(inp);
    let mut fr_data = [0u8; 32];
    fr_data.copy_from_slice(&hash);
    ZkScalar::new(fr_data)
}

impl SignatureScheme for EdDSA {
    type Pub = PublicKey;
    type Priv = PrivateKey;
    type Sig = Signature;
    fn generate_keys(seed: &[u8]) -> (PublicKey, PrivateKey) {
        let randomness = hash_to_fr(seed);
        let scalar = hash_to_fr(randomness.to_repr().as_ref());
        generate_keys(randomness, scalar)
    }
    fn sign(sk: &PrivateKey, message: &[u8]) -> Signature {
        let hash = hash_to_fr(message);
        sign::<crate::core::ZkHasher>(&sk, hash)
    }
    fn verify(pk: &PublicKey, message: &[u8], sig: &Signature) -> bool {
        let hash = hash_to_fr(message);
        verify::<crate::core::ZkHasher>(&pk, hash, &sig)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_key_compression() {
        let p1 = BASE.multiply(&ZkScalar::from(123_u64));
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
