pub mod ram;

use crate::config;
use ff::Field;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use zeekit::Fr;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ZkError {
    #[error("delta not found")]
    DeltaNotFound,
}

pub fn check_proof(
    vk: &ZkVerifierKey,
    prev_state: &ZkCompressedState,
    aux_data: &ZkCompressedState,
    next_state: &ZkCompressedState,
    proof: &ZkProof,
) -> bool {
    match vk {
        ZkVerifierKey::Groth16(vk) => {
            if let ZkProof::Groth16(proof) = proof {
                zeekit::groth16_verify(
                    vk,
                    prev_state.state_hash.0,
                    aux_data.state_hash.0,
                    next_state.state_hash.0,
                    proof,
                )
            } else {
                false
            }
        }
        #[cfg(test)]
        ZkVerifierKey::Dummy => {
            if let ZkProof::Dummy(result) = proof {
                *result
            } else {
                false
            }
        }
        _ => {
            unimplemented!()
        }
    }
}

// A single state cell
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ZkScalar(Fr);

impl From<u64> for ZkScalar {
    fn from(val: u64) -> Self {
        Self(Fr::from(val))
    }
}

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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZkState {
    height: u64,
    state_model: ZkStateModel,
    deltas: Vec<ZkStateBiDelta>,
    state: HashMap<u32, ZkScalar>,
}

// One-way delta
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ZkStateDelta(HashMap<u32, ZkScalar>);

// One-way delta
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ZkStateBiDelta {
    forth: ZkStateDelta,
    back: ZkStateDelta,
}

impl ZkState {
    pub fn height(&self) -> u64 {
        self.height
    }
    pub fn size(&self) -> u32 {
        self.state.len() as u32
    }
    pub fn new(height: u64, state_model: ZkStateModel, data: HashMap<u32, ZkScalar>) -> Self {
        Self {
            height,
            state_model,
            state: data,
            deltas: Vec::new(),
        }
    }
    pub fn as_delta(&self) -> ZkStateDelta {
        ZkStateDelta(self.state.clone())
    }
    pub fn push_delta(&mut self, patch: &ZkStateDelta) {
        let mut rev_delta = ZkStateDelta(HashMap::new());
        for (k, _) in patch.0.iter() {
            match self.state.get(k) {
                Some(prev_v) => rev_delta.0.insert(*k, *prev_v),
                None => rev_delta.0.insert(*k, ZkScalar(Fr::zero())),
            };
        }
        self.apply_delta(patch);
        self.deltas.insert(
            0,
            ZkStateBiDelta {
                forth: patch.clone(),
                back: rev_delta,
            },
        );
        self.deltas.truncate(config::NUM_STATE_DELTAS_KEEP);
    }
    pub fn apply_delta(&mut self, patch: &ZkStateDelta) {
        for (k, v) in patch.0.iter() {
            if v.0.is_zero().into() {
                self.state.remove(k);
            } else {
                self.state.insert(*k, *v);
            }
        }
        self.height += 1;
    }
    pub fn compress(&self) -> ZkCompressedState {
        let root = ZkScalar(ram::ZkRam::from_state(self).root());
        ZkCompressedState {
            height: self.height,
            state_hash: root,
            state_size: self.size(),
        }
    }
    pub fn rollback(&mut self) -> Result<(), ZkError> {
        if self.deltas.is_empty() {
            return Err(ZkError::DeltaNotFound);
        }
        let back = self.deltas.remove(0).back;
        self.apply_delta(&back);
        self.height -= 2; // Height is advanced when applying block, so step back by 2
        Ok(())
    }
    pub fn compress_prev_states(&self) -> Vec<ZkCompressedState> {
        let mut res = Vec::new();
        let mut curr = self.clone();
        while !curr.deltas.is_empty() {
            curr.rollback().unwrap();
            res.push(curr.compress());
        }
        res
    }
    pub fn delta_of(&self, away: usize) -> Result<ZkStateDelta, ZkError> {
        if away == 0 {
            return Ok(ZkStateDelta::default());
        }
        let mut acc = self
            .deltas
            .get(away - 1)
            .ok_or(ZkError::DeltaNotFound)?
            .forth
            .clone();
        for i in away - 1..0 {
            acc.combine(&self.deltas.get(i - 1).ok_or(ZkError::DeltaNotFound)?.forth);
        }
        Ok(acc)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct ZkCompressedState {
    height: u64,
    state_hash: ZkScalar,
    state_size: u32,
}

impl ZkCompressedState {
    pub fn height(&self) -> u64 {
        self.height
    }
    pub fn size(&self) -> u32 {
        self.state_size
    }
}

impl ZkStateDelta {
    pub fn new(data: HashMap<u32, ZkScalar>) -> Self {
        Self(data)
    }
    pub fn combine(&mut self, other: &Self) {
        for (k, v) in other.0.iter() {
            if v.0.is_zero().into() {
                self.0.remove(k);
            } else {
                self.0.insert(*k, *v);
            }
        }
    }
    pub fn size(&self) -> isize {
        let mut sz = 0isize;
        for (_, v) in self.0.iter() {
            if v.0.is_zero().into() {
                sz -= 1;
            } else {
                sz += 1;
            }
        }
        sz
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ZkVerifierKey {
    Groth16(Box<zeekit::Groth16VerifyingKey>),
    Plonk(u8),
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
pub enum ZkProof {
    Groth16(Box<zeekit::Groth16Proof>),
    Plonk(u8),
    #[cfg(test)]
    Dummy(bool),
}
