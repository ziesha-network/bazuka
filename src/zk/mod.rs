mod mimc;
pub mod ram;

use crate::crypto::Fr;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// A single state cell
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ZkScalar(Fr);

// Full state of a contract
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZkState(HashMap<u32, ZkScalar>);

impl ZkState {
    pub fn size(&self) -> u32 {
        self.0.len() as u32
    }
    pub fn root(&self) -> ZkScalar {
        ZkScalar(ram::ZkRam::from_state(self).root())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ZkVerifierKey(#[serde(with = "serde_bytes")] Vec<u8>);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ZkProof(#[serde(with = "serde_bytes")] Vec<u8>);
