use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy)]
pub struct U256(pub [u8; 32]);

impl Default for U256 {
    fn default() -> Self {
        U256::zero()
    }
}

impl U256 {
    pub fn zero() -> Self {
        Self([0; 32])
    }
    pub fn empty() -> Self {
        Self([0u8; 32])
    }
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut data = [0u8; 32];
        data[..bytes.len()].copy_from_slice(bytes);
        Self(data)
    }
    pub fn random<R: RngCore>(rng: &mut R) -> Self {
        let mut data = [0u8; 32];
        rng.fill_bytes(&mut data);
        Self(data)
    }
    pub fn to_bits(&self) -> [bool; 256] {
        let mut ret = [false; 256];
        for i in 0..256 {
            ret[i] = ((self.0[i / 8] >> (i % 8)) & 1) == 1;
        }
        ret
    }
    pub fn to_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn generate(data: &Vec<u8>) -> Self {
        let mut hasher = Sha3_256::new();
        hasher.update(data);
        Self(hasher.finalize().try_into().unwrap())
    }

    // Dummy implementation of a 512-bit hash function, used for generating
    // scalar and randomness of EdDSA signatures.
    pub fn generate_two(data: &Vec<u8>) -> (Self, Self) {
        let mut hasher = Sha3_256::new();
        hasher.update(data);
        let first: [u8; 32] = hasher.finalize().try_into().unwrap();

        let mut hasher = Sha3_256::new();
        hasher.update(data);
        hasher.update(data);
        let second: [u8; 32] = hasher.finalize().try_into().unwrap();

        (Self(first), Self(second))
    }
}
