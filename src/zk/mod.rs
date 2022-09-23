use crate::core::{hash::Hash, Hasher, Money, ZkHasher as ZkMainHasher};
use crate::crypto::{jubjub, ZkSignatureScheme};

use ff::PrimeField;
use num_bigint::BigUint;
use num_integer::Integer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

use thiserror::Error;

mod state;
pub use state::*;
pub mod groth16;
pub mod poseidon;

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MpnAccount {
    pub nonce: u64,
    pub address: jubjub::PointAffine,
    pub balance: Money,
}

lazy_static! {
    pub static ref CONTRACT_PAYMENT_STATE_MODEL: ZkStateModel = ZkStateModel::Struct {
        field_types: vec![
            ZkStateModel::Scalar, // index
            ZkStateModel::Scalar, // amount
            ZkStateModel::Scalar, // direction
            ZkStateModel::Scalar, // pub-x
            ZkStateModel::Scalar, // pub-y
        ],
    };
}

#[derive(Error, Debug)]
pub enum ZkError {
    #[error("delta not found")]
    DeltaNotFound,
    #[error("scalar bigger than u64")]
    ScalarBiggerThanU64,
}

#[derive(Debug, Clone, Default)]
pub struct ZkStateProof(Vec<ZkScalar>);

pub trait ZkHasher: Clone {
    const MAX_ARITY: usize;
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
        ZkVerifierKey::Groth16(vk) =>
        {
            #[allow(irrefutable_let_patterns)]
            if let ZkProof::Groth16(proof) = proof {
                groth16::groth16_verify(
                    vk,
                    prev_state.state_hash,
                    aux_data.state_hash,
                    next_state.state_hash,
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
    }
}

lazy_static! {
    static ref ZKSCALAR_MODULUS: BigUint = BigUint::from_str(
        "52435875175126190479447740508185965837690552500527637822603658699938581184513"
    )
    .unwrap();
}

#[derive(PrimeField, Serialize, Deserialize)]
#[PrimeFieldModulus = "52435875175126190479447740508185965837690552500527637822603658699938581184513"]
#[PrimeFieldGenerator = "7"]
#[PrimeFieldReprEndianness = "little"]
pub struct ZkScalar([u64; 4]);

pub fn hash_to_scalar(inp: &[u8]) -> ZkScalar {
    ZkScalar::new(&Hasher::hash(inp))
}

impl ZkScalar {
    pub fn new(num_le: &[u8]) -> Self {
        let bts = BigUint::from_bytes_le(num_le)
            .mod_floor(&ZKSCALAR_MODULUS)
            .to_bytes_le();
        let mut data = [0u8; 32];
        data[0..bts.len()].copy_from_slice(&bts);
        ZkScalar::from_repr_vartime(ZkScalarRepr(data)).unwrap()
    }
}

impl From<Money> for ZkScalar {
    fn from(m: Money) -> Self {
        let as_u64: u64 = m.into();
        Self::from(as_u64)
    }
}

impl TryInto<u64> for ZkScalar {
    type Error = ZkError;

    fn try_into(self) -> Result<u64, Self::Error> {
        if !self.to_repr().as_ref()[8..].iter().all(|d| *d == 0) {
            Err(ZkError::ScalarBiggerThanU64)
        } else {
            Ok(u64::from_le_bytes(
                self.to_repr().as_ref()[..8].try_into().unwrap(),
            ))
        }
    }
}

impl TryInto<Money> for ZkScalar {
    type Error = ZkError;

    fn try_into(self) -> Result<Money, Self::Error> {
        Ok(Money(self.try_into()?))
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

#[derive(Error, Debug)]
pub enum ZkLocatorError {
    #[error("locator pointing to nonexistent elements")]
    InvalidLocator,
}

impl ZkStateModel {
    pub fn is_valid<H: ZkHasher>(&self) -> bool {
        match self {
            ZkStateModel::Struct { field_types } => {
                if field_types.len() > H::MAX_ARITY {
                    false
                } else {
                    field_types.iter().all(|ft| ft.is_valid::<H>())
                }
            }
            ZkStateModel::List { item_type, .. } => item_type.is_valid::<H>(),
            ZkStateModel::Scalar => true,
        }
    }
    pub fn locate(&self, locator: &ZkDataLocator) -> Result<ZkStateModel, ZkLocatorError> {
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
                        return Err(ZkLocatorError::InvalidLocator);
                    }
                }
                ZkStateModel::Scalar => {
                    return Err(ZkLocatorError::InvalidLocator);
                }
            }
        }
        Ok(curr)
    }

    pub fn compress<H: ZkHasher>(
        &self,
        data: &ZkDataPairs,
    ) -> Result<ZkCompressedState, StateManagerError> {
        let mut builder = ZkStateBuilder::<H>::new(self.clone());
        builder.batch_set(&data.as_delta())?;
        builder.compress()
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

impl ZkDataLocator {
    pub fn index(&self, ind: u32) -> ZkDataLocator {
        let mut result = self.clone();
        result.0.push(ind);
        result
    }
}

impl std::fmt::Display for ZkDataLocator {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(|n| format!("{:x}", n))
                .collect::<Vec<_>>()
                .join("_")
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
            s.split('_')
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

#[derive(Debug, Clone, PartialEq, Eq, std::hash::Hash)]
pub struct PoseidonHasher;
impl ZkHasher for PoseidonHasher {
    const MAX_ARITY: usize = poseidon::MAX_ARITY;
    fn hash(vals: &[ZkScalar]) -> ZkScalar {
        poseidon::poseidon(vals)
    }
}

// Full state of a contract
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZkState {
    pub data: ZkDataPairs,
    pub rollbacks: Vec<ZkDeltaPairs>,
}

impl ZkState {
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
    pub state_hash: ZkScalar,
    pub state_size: u32,
}

impl ZkCompressedState {
    pub fn new(state_hash: ZkScalar, state_size: u32) -> Self {
        Self {
            state_hash,
            state_size,
        }
    }
    pub fn empty<H: ZkHasher>(data_type: ZkStateModel) -> Self {
        Self {
            state_hash: data_type.compress_default::<H>(),
            state_size: 0,
        }
    }
    pub fn size(&self) -> u32 {
        self.state_size
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ZkVerifierKey {
    Groth16(Box<groth16::Groth16VerifyingKey>),
    #[cfg(test)]
    Dummy,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ZkPaymentVerifierKey {
    pub verifier_key: ZkVerifierKey,
    pub log4_payment_capacity: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct MpnTransaction {
    pub nonce: u64,
    pub src_index: u32,
    pub dst_index: u32,
    pub dst_pub_key: jubjub::PublicKey,
    pub amount: Money,
    pub fee: Money,
    pub sig: jubjub::Signature,
}

impl Eq for MpnTransaction {}

impl PartialEq for MpnTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.hash() == other.hash()
    }
}

impl std::hash::Hash for MpnTransaction {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.hash().0.hash(state);
    }
}

impl MpnTransaction {
    pub fn verify(&self, addr: &jubjub::PublicKey) -> bool {
        jubjub::JubJub::<ZkMainHasher>::verify(addr, self.hash(), &self.sig)
    }
    pub fn sign(&mut self, sk: &jubjub::PrivateKey) {
        self.sig = jubjub::JubJub::<ZkMainHasher>::sign(sk, self.hash());
    }
    pub fn hash(&self) -> ZkScalar {
        ZkMainHasher::hash(&[
            ZkScalar::from(self.nonce),
            ZkScalar::from(self.src_index as u64),
            ZkScalar::from(self.dst_index as u64),
            ZkScalar::from(self.amount.0),
            ZkScalar::from(self.fee.0),
        ])
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ZkContract {
    pub initial_state: ZkCompressedState, // 32byte
    pub state_model: ZkStateModel,
    pub payment_functions: Vec<ZkPaymentVerifierKey>, // VK f(prev_state, io_txs (L1)) -> next_state
    pub functions: Vec<ZkVerifierKey>,                // Vec<VK> f(prev_state) -> next_state
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ZkProof {
    Groth16(Box<groth16::Groth16Proof>),
    #[cfg(test)]
    Dummy(bool),
}

#[cfg(test)]
mod test;
