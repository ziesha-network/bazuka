use super::{ZkState, ZkStateModel};
use ff::Field;
use std::collections::HashMap;
use zeekit::mimc;
use zeekit::Fr;

// A virtual RAM which represents the state of a contract
// based on a Sparse Merkle Tree.
#[derive(Debug, Clone)]
pub struct ZkRam {
    state_model: ZkStateModel,
    defaults: Vec<Fr>,
    layers: Vec<HashMap<u32, Fr>>,
}

#[derive(Debug, Clone, Default)]
pub struct Proof(Vec<Fr>);

impl ZkRam {
    pub fn from_state(state: &ZkState) -> Self {
        let mut r = Self::new(state.state_model);
        for (k, v) in state.state.iter() {
            r.set(*k, v.0);
        }
        r
    }
    pub fn new(state_model: ZkStateModel) -> Self {
        let mut defaults = vec![Fr::zero()];
        for i in 0..state_model.tree_depth as usize {
            defaults.push(mimc::mimc(&[defaults[i], defaults[i]]));
        }
        Self {
            state_model,
            defaults,
            layers: vec![HashMap::new(); state_model.tree_depth as usize + 1],
        }
    }
    pub fn root(&self) -> Fr {
        let depth = self.state_model.tree_depth as usize;
        *self.layers[depth].get(&0).unwrap_or(&self.defaults[depth])
    }
    fn get(&self, level: usize, index: u32) -> Fr {
        self.layers[level]
            .get(&index)
            .cloned()
            .unwrap_or(self.defaults[level])
    }
    pub fn prove(&self, mut index: u32) -> Proof {
        let mut proof = Vec::new();
        for level in 0..self.state_model.tree_depth as usize {
            let neigh = if index & 1 == 0 { index + 1 } else { index - 1 };
            proof.push(self.get(level, neigh as u32));
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
        for level in 0..(self.state_model.tree_depth as usize + 1) {
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
