use crate::core::digest::Digests;
use crate::core::{Hash, U256};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Header<H: Hash, N: Copy + Into<U256> + TryFrom<U256>> {
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

impl<H: Hash, N: Copy + Into<U256> + TryFrom<U256>> Header<H, N> {
    fn new(
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

    fn number(&self) -> &N {
        &self.number
    }

    fn block_root(&self) -> &H::Output {
        &self.block_root
    }

    fn state_root(&self) -> &H::Output {
        &self.state_root
    }

    fn parent_hash(&self) -> &H::Output {
        &self.parent_hash
    }

    fn digests(&self) -> &Digests {
        &self.digests
    }

    fn mut_digests(&mut self) -> &mut Digests {
        &mut self.digests
    }
}
