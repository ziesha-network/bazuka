use crate::config;
use ff::Field;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use zeekit::{mimc, Fr};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ZkError {
    #[error("delta not found")]
    DeltaNotFound,
}

#[derive(Debug, Clone, Default)]
pub struct ZkStateProof(Vec<ZkScalar>);

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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ZkStatePatch {
    Full(ZkStateFull),
    Delta(ZkStateDelta),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ZkDataType {
    // Allocate 1
    Scalar,
    // Allocate sum(size(data_type) for data_type in field_types)
    Struct {
        field_types: Vec<ZkDataType>,
    },
    // Allocate 2^log_size * size(item_type)
    List {
        log_size: u8,
        item_type: Box<ZkDataType>,
    },
}

impl ZkDataType {
    pub fn size(&self) -> usize {
        match self {
            ZkDataType::Scalar => 1,
            ZkDataType::Struct { field_types } => field_types.iter().map(|t| t.size()).sum(),
            ZkDataType::List {
                log_size,
                item_type,
            } => item_type.size() << log_size,
        }
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
    defaults: Vec<ZkScalar>,
    layers: Vec<HashMap<u32, ZkScalar>>,
}

// Full state
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZkStateFull {
    height: u64,
    state_model: ZkStateModel,
    state: HashMap<u32, ZkScalar>,
    deltas: Vec<ZkStateBiDelta>,
}

// One-way delta
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ZkStateDelta(HashMap<u32, ZkScalar>);

// Two-way delta
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
        self.layers[0].len() as u32
    }
    pub fn from_full(full: &ZkStateFull) -> Self {
        let mut tree = Self::new(full.height, full.state_model, full.state.clone());
        tree.deltas = full.deltas.clone();
        tree
    }
    pub fn new(height: u64, state_model: ZkStateModel, data: HashMap<u32, ZkScalar>) -> Self {
        let mut defaults = vec![ZkScalar::default()];
        for i in 0..state_model.tree_depth as usize {
            defaults.push(ZkScalar(mimc::mimc(&[defaults[i].0, defaults[i].0])));
        }
        let mut tree = Self {
            height,
            state_model,
            deltas: Vec::new(),
            defaults,
            layers: vec![HashMap::new(); state_model.tree_depth as usize + 1],
        };
        for (k, v) in data {
            tree.set(k, v);
        }
        tree
    }
    pub fn empty(state_model: ZkStateModel) -> Self {
        Self::new(0, state_model, HashMap::new())
    }
    pub fn genesis(state_model: ZkStateModel, data: HashMap<u32, ZkScalar>) -> Self {
        Self::new(1, state_model, data)
    }
    pub fn as_delta(&self) -> ZkStateDelta {
        ZkStateDelta(self.layers[0].clone())
    }
    pub fn as_full(&self) -> ZkStateFull {
        ZkStateFull {
            height: self.height,
            state_model: self.state_model,
            state: self.layers[0].clone(),
            deltas: self.deltas.clone(),
        }
    }
    pub fn push_delta(&mut self, patch: &ZkStateDelta) {
        let mut rev_delta = ZkStateDelta(HashMap::new());
        for (k, _) in patch.0.iter() {
            rev_delta.0.insert(*k, self.get(0, *k));
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
            self.set(*k, *v);
        }
        self.height += 1;
    }
    pub fn compress(&self) -> ZkCompressedState {
        let depth = self.state_model.tree_depth as usize;
        let root = *self.layers[depth].get(&0).unwrap_or(&self.defaults[depth]);
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
    fn get(&self, level: usize, index: u32) -> ZkScalar {
        self.layers[level]
            .get(&index)
            .cloned()
            .unwrap_or(self.defaults[level])
    }
    pub fn prove(&self, mut index: u32) -> ZkStateProof {
        let mut proof = Vec::new();
        for level in 0..self.state_model.tree_depth as usize {
            let neigh = if index & 1 == 0 { index + 1 } else { index - 1 };
            proof.push(self.get(level, neigh as u32));
            index >>= 1;
        }
        ZkStateProof(proof)
    }
    pub fn verify(
        mut index: u32,
        mut value: ZkScalar,
        proof: ZkStateProof,
        root: ZkScalar,
    ) -> bool {
        for p in proof.0 {
            value = ZkScalar(if index & 1 == 0 {
                mimc::mimc(&[value.0, p.0])
            } else {
                mimc::mimc(&[p.0, value.0])
            });
            index >>= 1;
        }
        value == root
    }
    pub fn set(&mut self, mut index: u32, mut value: ZkScalar) {
        for level in 0..(self.state_model.tree_depth as usize + 1) {
            if value.0.is_zero().into() {
                self.layers[level].remove(&index);
            } else {
                self.layers[level].insert(index, value);
            }
            let neigh = if index & 1 == 0 { index + 1 } else { index - 1 };
            let neigh_val = self.get(level, neigh);
            value = ZkScalar(if index & 1 == 0 {
                mimc::mimc(&[value.0, neigh_val.0])
            } else {
                mimc::mimc(&[neigh_val.0, value.0])
            });
            index >>= 1;
        }
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct ZeroTransaction;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ZkContract {
    pub initial_state: ZkCompressedState, // 32byte
    pub state_model: ZkStateModel,
    pub deposit_withdraw_function: ZkVerifierKey, // VK f(prev_state, io_txs (L1)) -> next_state
    pub functions: Vec<ZkVerifierKey>,            // Vec<VK> f(prev_state) -> next_state
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ZkProof {
    Groth16(Box<zeekit::Groth16Proof>),
    Plonk(u8),
    #[cfg(test)]
    Dummy(bool),
}
