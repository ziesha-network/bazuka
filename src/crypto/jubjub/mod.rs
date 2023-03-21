use crate::zk::{hash_to_scalar, ZkHasher, ZkScalar, ZkScalarRepr};
use ff::{Field, PrimeField};
use num_bigint::BigUint;
use num_integer::Integer;
use serde::{Deserialize, Serialize};
use std::ops::{AddAssign, MulAssign};

use super::{DeriveMpnAccountIndex, ZkSignatureScheme};

use std::str::FromStr;
use thiserror::Error;

mod curve;
pub use curve::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct JubJub<H: ZkHasher>(std::marker::PhantomData<H>);

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default, Eq, Hash)]
pub struct PublicKey(pub PointCompressed);

impl Into<Vec<ZkScalar>> for PublicKey {
    fn into(self) -> Vec<ZkScalar> {
        let decom = self.0.decompress();
        [decom.0, decom.1].into()
    }
}

impl DeriveMpnAccountIndex for PublicKey {
    fn mpn_account_index(&self, log4_account_capacity: u8) -> u64 {
        u64::from_le_bytes(self.0 .0.to_repr().as_ref()[0..8].try_into().unwrap())
            & ((1 << (2 * log4_account_capacity)) - 1)
    }
}

impl From<PrivateKey> for PublicKey {
    fn from(priv_key: PrivateKey) -> Self {
        Self(priv_key.public_key.compress())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Signature {
    pub r: PointAffine,
    pub s: ZkScalar,
}

impl Into<Vec<ZkScalar>> for Signature {
    fn into(self) -> Vec<ZkScalar> {
        [self.r.0, self.r.1, self.s].into()
    }
}

impl std::fmt::Display for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "jub{}", if self.0 .1 { 3 } else { 2 })?;
        for byte in self.0 .0.to_repr().as_ref().iter().rev() {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

impl PublicKey {
    pub fn is_on_curve(&self) -> bool {
        self.decompress().is_on_curve()
    }
    pub fn decompress(&self) -> PointAffine {
        self.0.decompress()
    }
}

impl FromStr for PublicKey {
    type Err = ParsePublicKeyError;
    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        if s.len() != 68 {
            return Err(ParsePublicKeyError::Invalid);
        }
        let oddity = if s.starts_with("jub3") {
            true
        } else if s.starts_with("jub2") {
            false
        } else {
            return Err(ParsePublicKeyError::Invalid);
        };
        s = &s[4..];
        let bytes = (0..32)
            .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16))
            .rev()
            .collect::<Result<Vec<u8>, std::num::ParseIntError>>()
            .map_err(|_| ParsePublicKeyError::Invalid)?;
        let mut repr = ZkScalar::ZERO.to_repr();
        repr.as_mut().clone_from_slice(&bytes);
        let as_opt: Option<ZkScalar> = ZkScalar::from_repr(repr).into();
        Ok(PublicKey(PointCompressed(
            as_opt.ok_or(ParsePublicKeyError::Invalid)?,
            oddity,
        )))
    }
}

impl<H: ZkHasher> ZkSignatureScheme for JubJub<H> {
    type Pub = PublicKey;
    type Priv = PrivateKey;
    type Sig = Signature;
    fn generate_keys(seed: &[u8]) -> (PublicKey, PrivateKey) {
        let randomness = hash_to_scalar(seed);
        let scalar = hash_to_scalar(randomness.to_repr().as_ref());
        let point = BASE.multiply(&scalar);
        let pk = PublicKey(point.compress());
        (
            pk,
            PrivateKey {
                public_key: point,
                randomness,
                scalar,
            },
        )
    }
    fn sign(sk: &PrivateKey, message: ZkScalar) -> Signature {
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
    fn verify(pk: &PublicKey, message: ZkScalar, sig: &Signature) -> bool {
        let pk = pk.0.decompress();

        if !pk.is_on_curve() || !sig.r.is_on_curve() {
            return false;
        }

        // h=H(R,A,M)
        let h = H::hash(&[sig.r.0, sig.r.1, pk.0, pk.1, message]);

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
    fn test_jubjub_public_key_compression() {
        let p1 = BASE.multiply(&ZkScalar::from(123_u64));
        let p2 = p1.compress().decompress();

        assert_eq!(p1, p2);
    }

    #[test]
    fn test_jubjub_signature_verification() {
        let (pk, sk) = JubJub::<crate::core::ZkHasher>::generate_keys(b"ABC");
        let msg = ZkScalar::from(123456);
        let fake_msg = ZkScalar::from(123457);
        let sig = JubJub::<crate::core::ZkHasher>::sign(&sk, msg);

        assert!(JubJub::<crate::core::ZkHasher>::verify(&pk, msg, &sig));
        assert!(!JubJub::<crate::core::ZkHasher>::verify(
            &pk, fake_msg, &sig
        ));
    }
}
