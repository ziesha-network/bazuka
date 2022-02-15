use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::convert::TryInto;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct U256([u8; 32]);

impl U256 {
    pub fn empty() -> Self {
        Self([0u8; 32])
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
