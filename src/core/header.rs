use crate::core::digest::{Digest, Digests};
use crate::core::{AutoSerialize, BlockNumber, Hash};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Header<H: Hash, N: BlockNumber + AutoSerialize> {
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

impl<H, N> Default for Header<H, N>
where
    H: Hash,
    N: BlockNumber + AutoSerialize,
{
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

impl<H, N> Header<H, N>
where
    H: Hash,
    N: BlockNumber + AutoSerialize,
{
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

    pub fn hash(&self) -> H::Output {
        H::hash(&bincode::serialize(&self).expect("convert header to bincode format"))
    }

    pub fn logs(&self) -> &[Digest] {
        self.digests.logs()
    }
}
