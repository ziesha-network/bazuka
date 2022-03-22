use crate::core::digest::{Digest, Digests};
use crate::core::Hash;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Header<H: Hash> {
    /// the parent hash
    pub parent_hash: H::Output,
    /// block number or block height
    pub number: u64,
    /// the root of state tired merkle tree
    pub state_root: H::Output,
    /// the merkle root of current block
    pub block_root: H::Output,
    /// aux data for consensus
    pub digests: Digests,
}

impl<H> Default for Header<H>
where
    H: Hash,
{
    fn default() -> Self {
        Header {
            parent_hash: H::Output::default(),
            number: 0,
            state_root: H::Output::default(),
            block_root: H::Output::default(),
            digests: Default::default(),
        }
    }
}

impl<H> Header<H>
where
    H: Hash,
{
    pub fn new(
        number: u64,
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

    pub fn hash(&self) -> H::Output {
        H::hash(&bincode::serialize(&self).expect("convert header to bincode format"))
    }

    pub fn logs(&self) -> &[Digest] {
        self.digests.logs()
    }
}
