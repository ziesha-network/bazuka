use std::ops::*;

use crate::core::number::U256;
use ff::Field;
use sha3::{Digest, Sha3_256};

use super::field::Fr;

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
            params.push(Fr::from_u256(&U256::from_le_bytes(&result)));
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
