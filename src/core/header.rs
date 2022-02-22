use crate::core::digest::Digests;
use crate::core::{Hash, U256};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Header<H: Hash, N: Default + Copy + Into<U256> + TryFrom<U256>> {
    /// the parent hash
    pub parent_hash: H::Output,
    /// block number or block height
    pub number: N,
    /// the root of state tired merkle tree
    pub state_root: H::Output,
    /// the merkle root of current block  
    pub block_root: H::Output,
    /// aux data for consensus
    pub digests: Digests,
}

impl<H: Hash, N: Default + Copy + Into<U256> + TryFrom<U256>> Default for Header<H, N> {
    fn default() -> Self {
        Header {
            parent_hash: H::Output::default(),
            number: Default::default(),
            state_root: H::Output::default(),
            block_root: H::Output::default(),
            digests: Default::default(),
        }
    }
}

impl<H: Hash, N: Default + Copy + Into<U256> + TryFrom<U256>> Header<H, N> {
    pub fn new(
        number: N,
        block_root: H::Output,
        state_root: H::Output,
        parent_hash: H::Output,
        digests: Digests,
    ) -> Self {
        Self {
            parent_hash,
            number,
            state_root,
            block_root,
            digests,
        }
    }
}
