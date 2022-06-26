use serde::{Deserialize, Serialize};

use crate::crypto::merkle::MerkleTree;
use crate::crypto::{SignatureScheme, ZkSignatureScheme};

use super::hash::Hash;
use super::header::Header;
use super::transaction::Transaction;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Block<H: Hash, S: SignatureScheme, ZS: ZkSignatureScheme> {
    pub header: Header<H>,
    pub body: Vec<Transaction<H, S, ZS>>,
}

impl<H: Hash, S: SignatureScheme, ZS: ZkSignatureScheme> Block<H, S, ZS> {
    pub fn merkle_tree(&self) -> MerkleTree<H> {
        MerkleTree::<H>::new(self.body.iter().map(|tx| tx.hash()).collect())
    }
}
