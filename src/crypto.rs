use ff::{Field, PrimeField};
use sha3::{Digest, Sha3_256};
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

pub struct PointAffine(Fr, Fr);

lazy_static! {
    static ref A: Fr = Fr::one().neg();
    static ref D: Fr = Fr::from_str_vartime(
        "12181644023421730124874158521699555681764249180949974110617291017600649128846"
    )
    .unwrap();
    static ref BASE: PointAffine = PointAffine(
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
        hasher.update(seed);
        let _result = hasher.finalize();
        MiMC { params: Vec::new() }
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
