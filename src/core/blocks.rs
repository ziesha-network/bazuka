use serde::{Deserialize, Serialize};

use super::header::Header;
use crate::core::{Hash, Transaction};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block<H: Hash> {
    pub header: Header<H>,
    pub body: Vec<Transaction>,
}

impl<H: Hash> Block<H> {}
