use serde::{Deserialize, Serialize};

use crate::core::hash::Sha3Hasher;
use crate::core::header::Header;
use crate::core::number::U256;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    // @todo export Sha3 and U256 as generic
    pub header: Header<Sha3Hasher, U256>,
    pub body: Vec<u8>,
}

impl Block {}
