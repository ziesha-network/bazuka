use crate::core::{Hash, Transaction};

pub struct MerkleTree<H: Hash> {
    root: H::Output,
}

impl<H: Hash> MerkleTree<H> {
    pub fn build(txs: &Vec<Transaction>) -> MerkleTree<H> {
        unimplemented!();
    }
}
