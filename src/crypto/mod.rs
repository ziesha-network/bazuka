use std::convert::TryInto;
use std::ops::*;
use std::str::FromStr;

use ff::{Field, PrimeField};
use num_bigint::BigUint;
use num_integer::Integer;
use sha3::{Digest, Sha3_256};

use crate::core::number::U256;

pub trait SignatureScheme {
    type Pub;
    type Priv;
    type Sig;
    fn generate_keys(seed: &[u8]) -> (Self::Pub, Self::Priv);
    fn sign(sk: &Self::Priv, msg: &[u8]) -> Self::Sig;
    fn verify(pk: &Self::Pub, msg: &[u8], sig: &Self::Sig) -> bool;
}

pub trait VerifiableRandomFunction {
    type Pub;
    type Priv;
    type Output;
    type Proof;
    fn generate(seed: &[u8]) -> (Self::Pub, Self::Priv);
    fn evaluate(sk: &Self::Priv, input: &[u8]) -> (Self::Output, Self::Proof);
    fn verify(pk: &Self::Pub, input: &[u8], output: &Self::Output, proof: &Self::Proof) -> bool;
}

#[derive(PrimeField)]
#[PrimeFieldModulus = "21888242871839275222246405745257275088548364400416034343698204186575808495617"]
#[PrimeFieldGenerator = "7"]
#[PrimeFieldReprEndianness = "little"]
pub struct Fr([u64; 4]);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointCompressed(Fr, bool);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointAffine(Fr, Fr);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointProjective(Fr, Fr, Fr);

impl AddAssign<&PointAffine> for PointAffine {
    fn add_assign(&mut self, other: &PointAffine) {
        if *self == *other {
            *self = self.double();
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
    pub fn double(&self) -> Self {
        let xx = (*A * self.0 * self.0 + self.1 * self.1).invert().unwrap();
        let yy = (Fr::one() + Fr::one() - *A * self.0 * self.0 - self.1 * self.1)
            .invert()
            .unwrap();
        return Self(
            ((self.0 * self.1) * xx).double(),
            (self.1 * self.1 - *A * self.0 * self.0) * yy,
        );
    }
    pub fn multiply(&self, scalar: &U256) -> Self {
        let mut result = PointProjective::zero();
        let self_proj = self.to_projective();
        for bit in scalar.to_bits().iter().rev() {
            result = result.double();
            if *bit {
                result.add_assign(&self_proj);
            }
        }
        result.to_affine()
    }
    pub fn to_projective(&self) -> PointProjective {
        PointProjective(self.0, self.1, Fr::one())
    }
    pub fn compress(&self) -> PointCompressed {
        PointCompressed(self.0, self.1.is_odd().into())
    }
}

impl PointCompressed {
    pub fn decompress(&self) -> PointAffine {
        let mut y = ((Fr::one() - *D * self.0.square()).invert().unwrap()
            * (Fr::one() - *A * self.0.square()))
        .sqrt()
        .unwrap();
        let is_odd: bool = y.is_odd().into();
        if self.1 != is_odd {
            y = y.neg();
        }
        PointAffine(self.0.clone(), y)
    }
}

impl AddAssign<&PointProjective> for PointProjective {
    fn add_assign(&mut self, other: &PointProjective) {
        if self.is_zero() {
            *self = *other;
            return;
        }
        if other.is_zero() {
            return;
        }
        if self.to_affine() == other.to_affine() {
            *self = self.double();
            return;
        }
        let a = self.2 * other.2; // A = Z1 * Z2
        let b = a.square(); // B = A^2
        let c = self.0 * other.0; // C = X1 * X2
        let d = self.1 * other.1; // D = Y1 * Y2
        let e = *D * c * d; // E = dC · D
        let f = b - e; // F = B − E
        let g = b + e; // G = B + E
        self.0 = a * f * ((self.0 + self.1) * (other.0 + other.1) - c - d);
        self.1 = a * g * (d - *A * c);
        self.2 = f * g;
    }
}

impl PointProjective {
    pub fn zero() -> Self {
        PointProjective(Fr::zero(), Fr::one(), Fr::zero())
    }
    pub fn is_zero(&self) -> bool {
        self.2.is_zero().into()
    }
    pub fn double(&self) -> PointProjective {
        if self.is_zero() {
            return PointProjective::zero();
        }
        let b = (self.0 + self.1).square();
        let c = self.0.square();
        let d = self.1.square();
        let e = *A * c;
        let f = e + d;
        let h = self.2.square();
        let j = f - h.double();
        PointProjective((b - c - d) * j, f * (e - d), f * j)
    }
    pub fn to_affine(&self) -> PointAffine {
        if self.is_zero() {
            return PointAffine::zero();
        }
        let zinv = self.2.invert().unwrap();
        PointAffine(self.0 * zinv, self.1 * zinv)
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

#[derive(Clone, Debug, PartialEq)]
pub struct PublicKey(PointCompressed);

#[derive(Clone)]
pub struct PrivateKey {
    public_key: PointAffine,
    randomness: U256,
    scalar: U256,
}

#[derive(Clone)]
pub struct Signature {
    r: PointAffine,
    s: U256,
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
    fn test_twisted_edwards_curve_ops() {
        // ((2G) + G) + G
        let mut a = BASE.double();
        a.add_assign(&BASE);
        a.add_assign(&BASE);

        // 2(2G)
        let b = BASE.double().double();

        assert_eq!(a, b);

        // G + G + G + G
        let mut c = BASE.clone();
        c.add_assign(&BASE);
        c.add_assign(&BASE);
        c.add_assign(&BASE);

        assert_eq!(b, c);

        // Check if projective points are working
        let mut pnt1 = BASE.to_projective().double().double();
        pnt1.add_assign(&BASE.to_projective());
        let mut pnt2 = BASE.double().double();
        pnt2.add_assign(&BASE);

        assert_eq!(pnt1.to_affine(), pnt2);
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
