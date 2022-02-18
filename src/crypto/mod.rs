use crate::core::U256;
use ff::{Field, PrimeField, PrimeFieldBits};
use num_bigint::BigUint;
use num_integer::Integer;
use sha3::{Digest, Sha3_256};
use std::convert::TryInto;
use std::ops::*;
use std::str::FromStr;

pub trait SignatureScheme<Pub, Priv, Sig> {
    fn generate() -> (Pub, Priv);
    fn sign(sk: Priv, msg: Vec<u8>) -> Sig;
    fn verify(pk: Pub, msg: Vec<u8>, sig: Sig) -> bool;
}

pub trait VerifiableRandomFunction<Pub, Priv, Output, Proof> {
    fn generate() -> (Pub, Priv);
    fn evaluate(sk: Priv, input: Vec<u8>) -> (Output, Proof);
    fn verify(pk: Pub, input: Vec<u8>, output: Output, proof: Proof) -> bool;
}

#[derive(PrimeField)]
#[PrimeFieldModulus = "21888242871839275222246405745257275088548364400416034343698204186575808495617"]
#[PrimeFieldGenerator = "7"]
#[PrimeFieldReprEndianness = "little"]
pub struct Fr([u64; 4]);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointAffine(Fr, Fr);

impl AddAssign for PointAffine {
    fn add_assign(&mut self, other: Self) {
        if *self == other {
            self.double();
            return;
        }
        let xx = (Fr::one() + *D * self.0 * other.0 * self.1 * other.1)
            .invert()
            .unwrap();
        let yy = (Fr::one() - *D * self.0 * other.0 * self.1 * other.1)
            .invert()
            .unwrap();
        *self = Self(
            (self.0 * other.1 + self.1 * other.0) * xx,
            (self.1 * other.1 - *A * self.0 * other.0) * yy,
        );
    }
}

impl PointAffine {
    pub fn zero() -> Self {
        Self(Fr::zero(), Fr::one())
    }
    pub fn double(&mut self) {
        let xx = (*A * self.0 * self.0 + self.1 * self.1).invert().unwrap();
        let yy = (Fr::one() + Fr::one() - *A * self.0 * self.0 - self.1 * self.1)
            .invert()
            .unwrap();
        *self = Self(
            ((self.0 * self.1) * xx).double(),
            (self.1 * self.1 - *A * self.0 * self.0) * yy,
        )
    }
    pub fn multiply(&mut self, scalar: &Vec<bool>) {
        let mut result = PointAffine::zero();
        for bit in scalar.iter().rev() {
            result.double();
            if *bit {
                result.add_assign(*self);
            }
        }
        *self = result;
    }
}

lazy_static! {
    static ref A: Fr = Fr::one().neg();
    static ref D: Fr = Fr::from_str_vartime(
        "12181644023421730124874158521699555681764249180949974110617291017600649128846"
    )
    .unwrap();
    pub static ref BASE: PointAffine = PointAffine(
        Fr::from_str_vartime(
            "9671717474070082183213120605117400219616337014328744928644933853176787189663"
        )
        .unwrap(),
        Fr::from_str_vartime(
            "16950150798460657717958625567821834550301663161624707787222815936182638968203"
        )
        .unwrap()
    );
    static ref ORDER: BigUint = BigUint::from_str(
        "2736030358979909402780800718157159386076813972158567259200215660948447373041"
    )
    .unwrap();
}

pub struct MiMC {
    params: Vec<Fr>,
}

impl MiMC {
    pub fn new(seed: &[u8]) -> MiMC {
        let mut hasher = Sha3_256::new();
        let mut params = Vec::new();
        hasher.update(seed);
        for _ in 0..90 {
            let result = hasher.finalize();
            let mut elem = Fr::zero();
            elem.0[0] = u64::from_le_bytes(result[..8].try_into().unwrap());
            elem.0[1] = u64::from_le_bytes(result[8..16].try_into().unwrap());
            elem.0[2] = u64::from_le_bytes(result[16..24].try_into().unwrap());
            elem.0[3] = u64::from_le_bytes(result[24..32].try_into().unwrap());
            params.push(elem);
            hasher = Sha3_256::new();
            hasher.update(result);
        }

        MiMC { params }
    }
    fn encrypt(&self, mut inp: Fr, k: &Fr) -> Fr {
        for c in self.params.iter() {
            let tmp = inp + c + k;
            inp = tmp * tmp;
            inp.mul_assign(&tmp);
        }
        inp.add_assign(k);
        inp
    }
    pub fn hash(&self, data: &Vec<Fr>) -> Fr {
        let mut digest = Fr::zero();
        for d in data.iter() {
            digest.add_assign(&self.encrypt(d.clone(), &digest));
        }
        digest
    }
}

pub struct EdDSA;

#[derive(Clone)]
pub struct PublicKey {
    point: PointAffine,
}

#[derive(Clone)]
pub struct PrivateKey {
    randomness: U256,
    scalar: U256,
}

#[derive(Clone)]
pub struct Signature {
    r: PointAffine,
    s: U256,
}

pub fn bits_to_u8(bits: &Vec<bool>) -> Vec<u8> {
    let mut bytes = Vec::new();
    for chunk in bits.chunks(8) {
        let mut byte = 0u8;
        for bit in chunk.iter().rev() {
            byte = byte << 1;
            byte = byte + (if *bit { 1 } else { 0 });
        }
        bytes.push(byte);
    }
    bytes
}

impl EdDSA {
    pub fn generate_keys(seed: &Vec<u8>) -> (PublicKey, PrivateKey) {
        let (randomness, scalar) = U256::generate_two(seed);
        let mut pk = BASE.clone();
        pk.multiply(&scalar.to_bits().to_vec());
        (PublicKey { point: pk }, PrivateKey { randomness, scalar })
    }
    pub fn sign(pk: PublicKey, sk: PrivateKey, message: &Vec<u8>) -> Signature {
        // r=H(b,M)
        let mut randomized_message = sk.randomness.to_bytes().to_vec();
        randomized_message.extend(message);
        let (r, _) = U256::generate_two(&randomized_message);

        // R=rB
        let mut rr = BASE.clone();
        rr.multiply(&r.to_bits().to_vec());

        // h=H(R,A,M)
        let mut inp = Vec::new();
        inp.extend(bits_to_u8(&rr.0.to_le_bits().into_iter().collect()));
        inp.extend(bits_to_u8(&rr.1.to_le_bits().into_iter().collect()));
        inp.extend(bits_to_u8(&pk.point.0.to_le_bits().into_iter().collect()));
        inp.extend(bits_to_u8(&pk.point.1.to_le_bits().into_iter().collect()));
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
            s: U256::from_bytes(&s.to_bytes_le()),
        }
    }
    pub fn verify(pk: PublicKey, message: &Vec<u8>, sig: Signature) -> bool {
        // h=H(R,A,M)
        let mut inp = Vec::new();
        inp.extend(bits_to_u8(&sig.r.0.to_le_bits().into_iter().collect()));
        inp.extend(bits_to_u8(&sig.r.1.to_le_bits().into_iter().collect()));
        inp.extend(bits_to_u8(&pk.point.0.to_le_bits().into_iter().collect()));
        inp.extend(bits_to_u8(&pk.point.1.to_le_bits().into_iter().collect()));
        inp.extend(message);
        let (h, _) = U256::generate_two(&inp);

        let mut sb = BASE.clone();
        sb.multiply(&sig.s.to_bits().to_vec());

        let mut r_plus_ha = pk.point.clone();
        r_plus_ha.multiply(&h.to_bits().to_vec());
        r_plus_ha.add_assign(sig.r);

        r_plus_ha == sb
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_twisted_edwards_curve_ops() {
        // ((2G) + G) + G
        let mut a = BASE.clone();
        a.double();
        a.add_assign(BASE.clone());
        a.add_assign(BASE.clone());

        // 2(2G)
        let mut b = BASE.clone();
        b.double();
        b.double();

        assert_eq!(a, b);

        // G + G + G + G
        let mut c = BASE.clone();
        c.add_assign(BASE.clone());
        c.add_assign(BASE.clone());
        c.add_assign(BASE.clone());

        assert_eq!(b, c);
    }
}
