use super::hash::Hash;
use crate::crypto::SignatureScheme;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Hash)]
pub struct ProofOfStake<S: SignatureScheme> {
    /// public-key of validator
    pub validator: S::Pub,
    /// when the validator started validating this block
    pub timestamp: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Hash)]
pub struct Header<H: Hash, S: SignatureScheme> {
    /// the parent hash
    pub parent_hash: H::Output,
    /// block number or block height
    pub number: u64,
    /// the merkle root of current block
    pub block_root: H::Output,
    /// aux data for Proof-of-Stake consensus
    pub proof_of_stake: ProofOfStake<S>,
}

impl<H: Hash, S: SignatureScheme> Header<H, S> {
    pub fn hash(&self) -> H::Output {
        H::hash(&bincode::serialize(&self).expect("convert header to bincode format"))
    }
}
