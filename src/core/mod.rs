use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::convert::TryInto;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct U256(pub [u8; 32]);

impl U256 {
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
}

pub type Signature = u8;
pub type Hash = U256;
pub type Money = u32;

impl Hash {
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

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum Address {
    Nowhere,
    PublicKey(u8),
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum Transaction {
    RegularSend {
        src: Address,
        dst: Address,
        amount: Money,
        sig: Signature,
    },
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct BlockHeader {
    pub index: usize,
    pub prev_hash: Hash,
    pub merkle_root: Hash,
    pub leader: Address,
    pub sig: Signature,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Block {
    pub header: BlockHeader,
    pub body: Vec<Transaction>,
}

impl Block {
    pub fn hash(&self) -> Hash {
        Hash::generate(&bincode::serialize(&self).unwrap())
    }
}
