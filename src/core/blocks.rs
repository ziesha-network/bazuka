use serde::{Deserialize, Serialize};

use crate::crypto::merkle::MerkleTree;
use crate::crypto::{SignatureScheme, VerifiableRandomFunction};

use super::hash::Hash;
use super::header::Header;
use super::transaction::Transaction;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Block<H: Hash, S: SignatureScheme, V: VerifiableRandomFunction> {
    pub header: Header<H, S>,
    pub body: Vec<Transaction<H, S, V>>,
}

impl<H: Hash, S: SignatureScheme, V: VerifiableRandomFunction> Block<H, S, V> {
    pub fn merkle_tree(&self) -> MerkleTree<H> {
        MerkleTree::<H>::new(self.body.iter().map(|tx| tx.hash()).collect())
    }
}
