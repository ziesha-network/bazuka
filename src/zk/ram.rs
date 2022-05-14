use super::ZkState;
use crate::config::LOG_ZK_RAM_SIZE;
use ff::Field;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use zeekit::mimc;
use zeekit::Fr;

// A virtual RAM which represents the state of a contract
// based on a Sparse Merkle Tree.
#[derive(Debug, Clone)]
pub struct ZkRam {
    layers: Vec<HashMap<u32, Fr>>,
}

// Represents delta of two ZkRam states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patch {
    for_root: Fr,
    new_root: Fr,
    delta: HashMap<u32, Fr>,
}

#[derive(Debug, Clone)]
pub struct Proof(pub [Fr; LOG_ZK_RAM_SIZE]);
impl Default for Proof {
    fn default() -> Self {
        Self([Fr::zero(); LOG_ZK_RAM_SIZE])
    }
}

impl Default for ZkRam {
    fn default() -> Self {
        Self::new()
    }
}

impl ZkRam {
    pub fn from_state(state: &ZkState) -> Self {
        let mut r = Self::new();
        for (k, v) in state.data.0.iter() {
            r.set(*k, v.0);
        }
        r
    }
    pub fn new() -> Self {
        Self {
            layers: vec![HashMap::new(); LOG_ZK_RAM_SIZE + 1],
        }
    }
    pub fn apply_patch(&mut self, patch: &Patch) {
        if patch.for_root == self.root() {
            let mut new_ram = self.clone();
            for (k, v) in patch.delta.iter() {
                new_ram.set(*k, *v);
            }
            if patch.new_root == new_ram.root() {
                *self = new_ram;
                return;
            }
        }
        panic!("Invalid patch!");
    }
    pub fn root(&self) -> Fr {
        *self.layers[LOG_ZK_RAM_SIZE].get(&0).expect("Tree empty!")
    }
    fn get(&self, level: usize, index: u32) -> Fr {
        self.layers[level]
            .get(&index)
            .cloned()
            .unwrap_or_else(Fr::zero)
    }
    pub fn prove(&self, mut index: u32) -> Proof {
        let mut proof = [Fr::zero(); LOG_ZK_RAM_SIZE];
        for (level, value) in proof.iter_mut().enumerate() {
            let neigh = if index & 1 == 0 { index + 1 } else { index - 1 };
            *value = self.get(level, neigh as u32);
            index >>= 1;
        }
        Proof(proof)
    }
    pub fn verify(mut index: u32, mut value: Fr, proof: Proof, root: Fr) -> bool {
        for p in proof.0 {
            value = if index & 1 == 0 {
                mimc::mimc(&[value, p])
            } else {
                mimc::mimc(&[p, value])
            };
            index >>= 1;
        }
        value == root
    }
    pub fn set(&mut self, mut index: u32, mut value: Fr) {
        for level in 0..(LOG_ZK_RAM_SIZE + 1) {
            self.layers[level].insert(index, value);
            let neigh = if index & 1 == 0 { index + 1 } else { index - 1 };
            let neigh_val = self.get(level, neigh);
            value = if index & 1 == 0 {
                mimc::mimc(&[value, neigh_val])
            } else {
                mimc::mimc(&[neigh_val, value])
            };
            index >>= 1;
        }
    }
}
