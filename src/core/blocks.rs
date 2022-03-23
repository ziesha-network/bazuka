use serde::{Deserialize, Serialize};

use crate::crypto::SignatureScheme;

use super::hash::Hash;
use super::header::Header;
use super::transaction::Transaction;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block<H: Hash, S: SignatureScheme> {
    pub header: Header<H>,
    pub body: Vec<Transaction<S>>,
}

impl<H: Hash, S: SignatureScheme> Block<H, S> {}
