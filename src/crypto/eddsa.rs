use std::ops::*;

use ff::PrimeField;
use num_bigint::BigUint;
use num_integer::Integer;
use serde::{Deserialize, Serialize};

use super::curve::*;
use super::SignatureScheme;
use crate::core::number::U256;

pub struct EdDSA;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PublicKey(pub PointCompressed);

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
    type Pub = PublicKey;
    type Priv = PrivateKey;
    type Sig = Signature;
    fn generate_keys(seed: &[u8]) -> (PublicKey, PrivateKey) {
        let (randomness, scalar) = U256::generate_two(seed);
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
    fn verify(pk: &PublicKey, message: &[u8], sig: &Signature) -> bool {
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
