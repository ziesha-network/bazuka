use rust_randomx::{Difficulty, Output};

use super::hash::Hash;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ProofOfWork {
    /// when the miner started mining this block
    pub timestamp: u32,
    /// difficulty target
    pub target: u32,
    /// arbitrary data
    pub nonce: u64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Header<H: Hash> {
    /// the parent hash
    pub parent_hash: H::Output,
    /// block number or block height
    pub number: u64,
    /// the merkle root of current block
    pub block_root: H::Output,

    /// aux data for Proof-of-Work consensus
    pub proof_of_work: ProofOfWork,
}

impl<H: Hash> Header<H> {
    pub fn hash(&self) -> H::Output {
        H::hash(&bincode::serialize(&self).expect("convert header to bincode format"))
    }

    fn pow_hash(&self, key: &[u8]) -> Output {
        let bin = bincode::serialize(&self).expect("convert header to bincode format");
        crate::consensus::pow::hash(key, &bin)
    }

    fn leading_zeros(&self, key: &[u8]) -> u8 {
        self.pow_hash(key).leading_zeros() as u8
    }

    // Approximate number of hashes run in order to generate this block

    pub fn power(&self, key: &[u8]) -> u64 {
        1u64 << self.leading_zeros(key)
    }

    pub fn meets_target(&self, key: &[u8]) -> bool {
        self.pow_hash(key)
            .meets_difficulty(Difficulty::new(self.proof_of_work.target))
    }
}
