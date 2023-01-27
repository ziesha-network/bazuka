use crate::consensus::pow::Difficulty;

use super::hash::Hash;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Hash)]
pub struct ProofOfWork {
    /// when the miner started mining this block
    pub timestamp: u32,
    /// difficulty target
    pub target: Difficulty,
    /// arbitrary data
    pub nonce: u64,
    /// commulative power
    pub comm_power: u128,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Hash)]
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
        self.proof_of_work.target.power()
    }

    pub fn meets_target(&self, key: &[u8]) -> bool {
        let bin = bincode::serialize(&self).expect("convert header to bincode format");
        crate::consensus::pow::meets_difficulty(key, &bin, self.proof_of_work.target)
    }
}
