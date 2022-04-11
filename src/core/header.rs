#[cfg(feature = "pos")]
use super::digest::{Digest, Digests};

use super::hash::Hash;

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

    /// aux data for Proof-of-Stake consensus
    #[cfg(feature = "pos")]
    pub digests: Digests,

    /// proof-of-work nonce
    #[cfg(feature = "pow")]
    pub nonce: u32,
}

impl<H: Hash> Default for Header<H> {
    fn default() -> Self {
        Header {
            parent_hash: H::Output::default(),
            number: 0,
            state_root: H::Output::default(),
            block_root: H::Output::default(),

            #[cfg(feature = "pos")]
            digests: Default::default(),

            #[cfg(feature = "pow")]
            nonce: 0,
        }
    }
}

impl<H: Hash> Header<H> {
    pub fn hash(&self) -> H::Output {
        H::hash(&bincode::serialize(&self).expect("convert header to bincode format"))
    }

    // Approximate number of hashes run in order to generate this block
    #[cfg(feature = "pow")]
    pub fn power(&self) -> u64 {
        let bin = bincode::serialize(&self).expect("convert header to bincode format");
        1u64 << crate::consensus::pow::leading_zeros(b"key", &bin)
    }

    #[cfg(feature = "pos")]
    pub fn logs(&self) -> &[Digest] {
        self.digests.logs()
    }
}
