use ff::{Field, PrimeField};
use sha3::{Digest, Sha3_256};
use std::convert::TryInto;
use std::ops::*;

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

#[derive(Debug, Clone, PartialEq)]
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
