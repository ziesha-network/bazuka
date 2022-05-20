pub mod ram;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use zeekit::Fr;

pub fn check_proof(
    vk: &ZkVerifierKey,
    _prev_state: &ZkCompressedState,
    _next_state: &ZkCompressedState,
    _proof: &ZkProof,
) -> bool {
    match vk {
        #[cfg(test)]
        ZkVerifierKey::Dummy => _proof.0.len() > 0,
        _ => unimplemented!(),
    }
}

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

impl ZkStateModel {
    pub fn new(leaf_size: u32, tree_depth: u8) -> Self {
        Self {
            leaf_size,
            tree_depth,
        }
    }
}

// Full state of a contract
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ZkState(HashMap<u32, ZkScalar>);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZkStateDelta(HashMap<u32, ZkScalar>);

impl ZkState {
    pub fn as_delta(&self) -> ZkStateDelta {
        ZkStateDelta(self.0.clone())
    }
    pub fn apply_patch(&mut self, patch: &ZkStateDelta) {
        self.0.extend(patch.0.iter());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ZkCompressedState {
    state_hash: ZkScalar,
    state_size: u32,
}

impl ZkState {
    pub fn size(&self) -> u32 {
        self.0.len() as u32
    }
}

impl ZkState {
    pub fn compress(&self, _model: ZkStateModel) -> ZkCompressedState {
        let root = ZkScalar(ram::ZkRam::from_state(self).root());
        ZkCompressedState {
            state_hash: root,
            state_size: self.size(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ZkVerifierKey {
    Groth16(#[serde(with = "serde_bytes")] Vec<u8>),
    #[cfg(test)]
    Dummy,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ZkContract {
    pub initial_state: ZkCompressedState, // 32byte
    pub state_model: ZkStateModel,
    pub deposit_withdraw: ZkVerifierKey, // VK f(prev_state, io_txs (L1)) -> next_state
    pub update: Vec<ZkVerifierKey>,      // Vec<VK> f(prev_state) -> next_state
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ZkProof(#[serde(with = "serde_bytes")] Vec<u8>);
