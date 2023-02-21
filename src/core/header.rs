use super::hash::Hash;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Hash)]
pub struct ProofOfWork {
    /// when the miner started mining this block
    pub timestamp: u32,
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
}
