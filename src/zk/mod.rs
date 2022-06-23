use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use zeekit::Fr;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ZkError {
    #[error("delta not found")]
    DeltaNotFound,
}

#[derive(Debug, Clone, Default)]
pub struct ZkStateProof(Vec<ZkScalar>);

pub trait ZkHasher: Clone {
    fn hash(vals: &[ZkScalar]) -> ZkScalar;
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
pub struct ZkScalar(pub Fr);

impl From<u64> for ZkScalar {
    fn from(val: u64) -> Self {
        Self(Fr::from(val))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ZkStatePatch {
    Full(ZkState),
    Delta(ZkDeltaPairs),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ZkStateModel {
    // Allocate 1
    Scalar,
    // Allocate sum(size(data_type) for data_type in field_types)
    Struct {
        field_types: Vec<ZkStateModel>,
    },
    // Allocate 4^log4_size * size(item_type)
    List {
        log4_size: u8,
        item_type: Box<ZkStateModel>,
    },
}

impl ZkStateModel {
    pub fn locate(&self, locator: &ZkDataLocator) -> ZkStateModel {
        let mut curr = self.clone();
        for l in locator.0.iter() {
            match curr {
                ZkStateModel::Struct { field_types } => {
                    curr = field_types[*l as usize].clone();
                }
                ZkStateModel::List {
                    item_type,
                    log4_size,
                } => {
                    if *l < 1 << (2 * log4_size) {
                        curr = *item_type.clone();
                    } else {
                        panic!();
                    }
                }
                ZkStateModel::Scalar => {
                    panic!();
                }
            }
        }
        curr
    }
    pub fn compress_default<H: ZkHasher>(&self) -> ZkScalar {
        match self {
            ZkStateModel::Scalar => ZkScalar::default(),
            ZkStateModel::Struct { field_types } => {
                let mut vals = vec![];
                for f in field_types.iter() {
                    vals.push(f.compress_default::<H>());
                }
                H::hash(&vals)
            }
            ZkStateModel::List {
                item_type,
                log4_size,
            } => {
                let mut root_default = item_type.compress_default::<H>();
                for _ in 0..*log4_size {
                    root_default =
                        H::hash(&[root_default, root_default, root_default, root_default])
                }
                root_default
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash)]
pub struct ZkDataLocator(pub Vec<u32>);

impl std::fmt::Display for ZkDataLocator {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(|n| format!("{:x}", n))
                .collect::<Vec<_>>()
                .join("-")
        )?;
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum ParseZkDataLocatorError {
    #[error("locator invalid")]
    Invalid,
}

impl std::str::FromStr for ZkDataLocator {
    type Err = ParseZkDataLocatorError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(
            s.split("-")
                .map(|s| u32::from_str_radix(s, 16))
                .collect::<Result<Vec<u32>, _>>()
                .map_err(|_| ParseZkDataLocatorError::Invalid)?,
        ))
    }
}

impl Eq for ZkDataLocator {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ZkDataPairs(pub HashMap<ZkDataLocator, ZkScalar>);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ZkDeltaPairs(pub HashMap<ZkDataLocator, Option<ZkScalar>>);

impl ZkDeltaPairs {
    pub fn size(&self) -> isize {
        self.0.len() as isize // TODO: Really?
    }
}

impl ZkDataPairs {
    pub fn as_delta(&self) -> ZkDeltaPairs {
        ZkDeltaPairs(
            self.0
                .clone()
                .into_iter()
                .map(|(k, v)| (k, Some(v)))
                .collect(),
        )
    }
    pub fn size(&self) -> usize {
        self.0.len() as usize
    }
}

#[derive(Clone)]
pub struct MimcHasher;
impl ZkHasher for MimcHasher {
    fn hash(vals: &[ZkScalar]) -> ZkScalar {
        ZkScalar(zeekit::mimc::mimc(
            &vals.iter().map(|v| v.0).collect::<Vec<_>>(),
        ))
    }
}

// Full state of a contract
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZkState {
    pub data: ZkDataPairs,
    pub rollbacks: Vec<ZkDeltaPairs>,
}

impl ZkState {
    pub fn compress<H: ZkHasher>(&self, model: ZkStateModel) -> ZkCompressedState {
        crate::blockchain::compress_state::<H>(model, self.data.clone()).unwrap()
    }
    pub fn push_delta(&mut self, delta: &ZkDeltaPairs) {
        let mut rollback = ZkDeltaPairs::default();
        for loc in delta.0.keys() {
            rollback
                .0
                .insert(loc.clone(), self.data.0.get(loc).cloned());
        }
        self.apply_delta(delta);
        self.rollbacks.push(rollback);
    }
    pub fn apply_delta(&mut self, delta: &ZkDeltaPairs) {
        for (loc, val) in delta.0.iter() {
            if let Some(val) = val {
                self.data.0.insert(loc.clone(), *val);
            } else {
                self.data.0.remove(loc);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct ZkCompressedState {
    pub height: u64,
    pub state_hash: ZkScalar,
    pub state_size: u32,
}

impl ZkCompressedState {
    pub fn new(height: u64, state_hash: ZkScalar, state_size: u32) -> Self {
        Self {
            height,
            state_hash,
            state_size,
        }
    }
    pub fn empty<H: ZkHasher>(data_type: ZkStateModel) -> Self {
        Self {
            height: 0,
            state_hash: data_type.compress_default::<H>(),
            state_size: 0,
        }
    }
    pub fn height(&self) -> u64 {
        self.height
    }
    pub fn size(&self) -> u32 {
        self.state_size
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ZkVerifierKey {
    Groth16(Box<zeekit::Groth16VerifyingKey>),
    Plonk(u8),
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
    Dummy(bool),
}
