use rust_randomx::Difficulty;

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

    // Approximate number of hashes run in order to generate this block
    pub fn power(&self) -> u128 {
        Difficulty::new(self.proof_of_work.target).power()
    }

    pub fn meets_target(&self, key: &[u8]) -> bool {
        let bin = bincode::serialize(&self).expect("convert header to bincode format");
        crate::consensus::pow::hash(key, &bin)
            .meets_difficulty(Difficulty::new(self.proof_of_work.target))
    }
}
