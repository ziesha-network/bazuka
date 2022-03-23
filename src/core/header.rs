use crate::core::digest::{Digest, Digests};
use crate::core::{Config, Hash};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Header<C: Config> {
    /// the parent hash
    pub parent_hash: <C::Hasher as Hash>::Output,
    /// block number or block height
    pub number: u64,
    /// the root of state tired merkle tree
    pub state_root: <C::Hasher as Hash>::Output,
    /// the merkle root of current block
    pub block_root: <C::Hasher as Hash>::Output,
    /// aux data for consensus
    pub digests: Digests,
}

impl<C> Default for Header<C>
where
    C: Config,
{
    fn default() -> Self {
        Header {
            parent_hash: <C::Hasher as Hash>::Output::default(),
            number: 0,
            state_root: <C::Hasher as Hash>::Output::default(),
            block_root: <C::Hasher as Hash>::Output::default(),
            digests: Default::default(),
        }
    }
}

impl<C> Header<C>
where
    C: Config,
{
    pub fn new(
        number: u64,
        block_root: <C::Hasher as Hash>::Output,
        state_root: <C::Hasher as Hash>::Output,
        parent_hash: <C::Hasher as Hash>::Output,
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

    pub fn hash(&self) -> <C::Hasher as Hash>::Output {
        C::Hasher::hash(&bincode::serialize(&self).expect("convert header to bincode format"))
    }

    pub fn logs(&self) -> &[Digest] {
        self.digests.logs()
    }
}
