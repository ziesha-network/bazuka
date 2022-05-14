use serde::{Deserialize, Serialize};

use crate::crypto::merkle::MerkleTree;
use crate::crypto::SignatureScheme;

use super::hash::Hash;
use super::header::Header;
use super::transaction::Transaction;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block<H: Hash, S: SignatureScheme> {
    pub header: Header<H>,
    pub body: Vec<Transaction<H, S>>,
}

impl<H: Hash, S: SignatureScheme> Block<H, S> {
    pub fn merkle_tree(&self) -> MerkleTree<H> {
        MerkleTree::<H>::new(self.body.iter().map(|tx| tx.hash()).collect())
    }
}
