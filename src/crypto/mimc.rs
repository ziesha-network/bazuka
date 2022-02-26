use std::ops::*;

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
            let elem = Fr::zero();
            // elem.0[0] = u64::from_le_bytes(result[..8].try_into().unwrap());
            // elem.0[1] = u64::from_le_bytes(result[8..16].try_into().unwrap());
            // elem.0[2] = u64::from_le_bytes(result[16..24].try_into().unwrap());
            // elem.0[3] = u64::from_le_bytes(result[24..32].try_into().unwrap());
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
