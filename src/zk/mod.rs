pub mod ram;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use zeekit::Fr;

// A single state cell
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ZkScalar(Fr);

// Each leaf of the target sparse merkle tree will be the
// result of consecutive hash of `leaf_size` cells.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ZkStateModel {
    leaf_size: u32,
    tree_depth: u8,
}

// Full state of a contract
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZkStateData(HashMap<u32, ZkScalar>);

pub struct ZkState {
    _model: ZkStateModel,
    data: ZkStateData,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ZkCompressedState {
    state_hash: ZkScalar,
    state_size: u32,
}

impl ZkStateData {
    pub fn size(&self) -> u32 {
        self.0.len() as u32
    }
}

impl ZkState {
    pub fn new(model: ZkStateModel, data: ZkStateData) -> Self {
        Self {
            _model: model,
            data,
        }
    }
    pub fn compress(&self) -> ZkCompressedState {
        let root = ZkScalar(ram::ZkRam::from_state(self).root());
        ZkCompressedState {
            state_hash: root,
            state_size: self.data.size(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ZkVerifierKey(#[serde(with = "serde_bytes")] Vec<u8>);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ZkProof(#[serde(with = "serde_bytes")] Vec<u8>);
