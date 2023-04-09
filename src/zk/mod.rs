use crate::core::{hash::Hash, Amount, Hasher, Money, TokenId, ZkHasher as ZkMainHasher};
use crate::crypto::{jubjub, DeriveMpnAccountIndex, ZkSignatureScheme};

use ff::{Field, PrimeField};
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

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
pub struct LruCache<K: std::hash::Hash + Clone, V> {
    capacity: usize,
    data: HashMap<K, V>,
    keys: VecDeque<K>,
}

impl<K: std::hash::Hash + Clone + Eq + std::fmt::Debug, V: std::fmt::Debug> LruCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            data: HashMap::new(),
            keys: VecDeque::new(),
        }
    }
    pub fn get(&mut self, key: &K) -> Option<&V> {
        if let Some(v) = self.data.get(key) {
            self.keys.retain(|k| *k != *key);
            self.keys.push_back(key.clone());
            Some(v)
        } else {
            None
        }
    }
    pub fn insert(&mut self, key: K, value: V) {
        if !self.data.contains_key(&key) {
            self.keys.push_back(key.clone());
            self.data.insert(key, value);
            while self.keys.len() > self.capacity {
                if let Some(k) = self.keys.pop_front() {
                    self.data.remove(&k);
                }
            }
        } else {
            self.get(&key);
            self.data.insert(key, value);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MpnAccount {
    pub tx_nonce: u32,       // Increased on MpnTransactions
    pub withdraw_nonce: u32, // Increased on MpnWithdrawals
    pub address: jubjub::PointAffine,
    pub tokens: HashMap<u64, Money>,
}

impl MpnAccount {
    pub fn tokens_hash<H: ZkHasher>(&self, log4_token_capacity: u8) -> ZkScalar {
        let state_model = ZkStateModel::List {
            log4_size: log4_token_capacity,
            item_type: Box::new(ZkStateModel::Struct {
                field_types: vec![
                    ZkStateModel::Scalar, // Token-Id
                    ZkStateModel::Scalar, // Balance
                ],
            }),
        };
        let mut state_builder = ZkStateBuilder::<H>::new(state_model);
        for (i, money) in self.tokens.iter() {
            state_builder
                .batch_set(&ZkDeltaPairs(
                    [
                        (
                            ZkDataLocator(vec![*i as u64, 0]),
                            Some(money.token_id.into()),
                        ),
                        (ZkDataLocator(vec![*i as u64, 1]), Some(money.amount.into())),
                    ]
                    .into(),
                ))
                .unwrap();
        }
        state_builder.compress().unwrap().state_hash
    }
    pub fn find_token_index(
        &self,
        log4_token_capacity: u8,
        token_id: TokenId,
        empty_allowed: bool,
    ) -> Option<u64> {
        for (ind, money) in self.tokens.iter() {
            if money.token_id == token_id {
                return Some(*ind);
            }
        }
        if empty_allowed {
            for ind in 0..1 << (2 * log4_token_capacity) {
                if !self.tokens.contains_key(&ind) {
                    return Some(ind);
                }
            }
        }
        None
    }
}

// Amount is passed by default
lazy_static! {
    pub static ref MPN_DEPOSIT_STATE_MODEL: ZkStateModel = ZkStateModel::Struct {
        field_types: vec![
            ZkStateModel::Scalar, // pub-x
            ZkStateModel::Scalar, // pub-y
        ],
    };
}

// Amount and fee are passed by default
lazy_static! {
    pub static ref MPN_WITHDRAW_STATE_MODEL: ZkStateModel = ZkStateModel::Struct {
        field_types: vec![
            ZkStateModel::Scalar, // pub-x
            ZkStateModel::Scalar, // pub-y
            ZkStateModel::Scalar, // nonce
            ZkStateModel::Scalar, // sig_r_x
            ZkStateModel::Scalar, // sig_r_y
            ZkStateModel::Scalar, // sig_s
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

pub trait ZkHasher: Clone + Default {
    const MAX_ARITY: usize;
    fn hash(vals: &[ZkScalar]) -> ZkScalar;
}

pub fn check_proof(
    vk: &ZkVerifierKey,
    prev_height: u64,
    prev_state: ZkScalar,
    calldata: ZkScalar,
    next_state: ZkScalar,
    proof: &ZkProof,
) -> bool {
    match vk {
        ZkVerifierKey::Groth16(vk) =>
        {
            #[allow(irrefutable_let_patterns)]
            if let ZkProof::Groth16(proof) = proof {
                groth16::groth16_verify(vk, prev_height, prev_state, calldata, next_state, proof)
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

impl std::hash::Hash for ZkScalar {
    fn hash<Hasher>(&self, state: &mut Hasher)
    where
        Hasher: std::hash::Hasher,
    {
        self.0.hash(state);
        state.finish();
    }
}

pub fn hash_to_scalar(inp: &[u8]) -> ZkScalar {
    ZkScalar::new(&Hasher::hash(inp))
}

impl std::fmt::Display for ZkScalar {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let rep = self
            .to_repr()
            .as_ref()
            .iter()
            .rev()
            .cloned()
            .collect::<Vec<u8>>();
        write!(f, "0x{}", hex::encode(rep))
    }
}

#[derive(Debug, Error)]
pub enum ParseZkScalarError {
    #[error("scalar invalid")]
    Invalid,
}

impl std::str::FromStr for ZkScalar {
    type Err = ParseZkScalarError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with("0x") {
            return Err(Self::Err::Invalid);
        }
        let bytes = hex::decode(&s[2..])
            .map_err(|_| Self::Err::Invalid)?
            .into_iter()
            .rev()
            .collect::<Vec<u8>>();
        if bytes.len() != 32 {
            return Err(Self::Err::Invalid);
        }
        let mut ret = Self::ZERO.to_repr();
        ret.as_mut().copy_from_slice(&bytes);
        let opt: Option<Self> = Self::from_repr(ret).into();
        opt.ok_or(Self::Err::Invalid)
    }
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

impl From<Amount> for ZkScalar {
    fn from(m: Amount) -> Self {
        let as_u64: u64 = m.into();
        Self::from(as_u64)
    }
}

impl From<TokenId> for ZkScalar {
    fn from(m: TokenId) -> Self {
        match m {
            TokenId::Null => Self::ZERO,
            TokenId::Ziesha => Self::ONE,
            TokenId::Custom(id) => id,
        }
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

impl TryInto<u32> for ZkScalar {
    type Error = ZkError;

    fn try_into(self) -> Result<u32, Self::Error> {
        if !self.to_repr().as_ref()[4..].iter().all(|d| *d == 0) {
            Err(ZkError::ScalarBiggerThanU64)
        } else {
            Ok(u32::from_le_bytes(
                self.to_repr().as_ref()[..4].try_into().unwrap(),
            ))
        }
    }
}

impl TryInto<Amount> for ZkScalar {
    type Error = ZkError;

    fn try_into(self) -> Result<Amount, Self::Error> {
        Ok(Amount(self.try_into()?))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ZkStatePatch {
    Full(ZkState),
    Delta(ZkDeltaPairs),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct ZkDataLocator(pub Vec<u64>);

impl ZkDataLocator {
    pub fn index(&self, ind: u64) -> ZkDataLocator {
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
                .map(|s| u64::from_str_radix(s, 16))
                .collect::<Result<Vec<u64>, _>>()
                .map_err(|_| ParseZkDataLocatorError::Invalid)?,
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ZkDataPairs(pub HashMap<ZkDataLocator, ZkScalar>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
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

lazy_static! {
    pub static ref POSEIDON_CACHE: Arc<Mutex<LruCache<Vec<ZkScalar>, ZkScalar>>> =
        Arc::new(Mutex::new(LruCache::new(64)));
}

#[derive(Debug, Clone, PartialEq, Eq, std::hash::Hash, Default)]
pub struct PoseidonHasher;
impl ZkHasher for PoseidonHasher {
    const MAX_ARITY: usize = poseidon::MAX_ARITY;
    fn hash(vals: &[ZkScalar]) -> ZkScalar {
        let mut h = POSEIDON_CACHE.lock().unwrap();
        let vals_vec = vals.to_vec();
        if let Some(v) = h.get(&vals_vec) {
            *v
        } else {
            let v = poseidon::poseidon(vals);
            h.insert(vals_vec, v);
            v
        }
    }
}

// Full state of a contract
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ZkCompressedState {
    pub state_hash: ZkScalar,
    pub state_size: u64,
}

impl ZkCompressedState {
    pub fn new(state_hash: ZkScalar, state_size: u64) -> Self {
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
    pub fn size(&self) -> u64 {
        self.state_size
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZkVerifierKey {
    Groth16(Box<groth16::Groth16VerifyingKey>),
    #[cfg(test)]
    Dummy,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZkMultiInputVerifierKey {
    pub verifier_key: ZkVerifierKey,
    pub log4_payment_capacity: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZkSingleInputVerifierKey {
    pub verifier_key: ZkVerifierKey,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct MpnTransaction {
    pub nonce: u32,
    pub src_pub_key: jubjub::PublicKey,
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
    pub fn src_index(&self, log4_account_capacity: u8) -> u64 {
        self.src_pub_key.mpn_account_index(log4_account_capacity)
    }
    pub fn dst_index(&self, log4_account_capacity: u8) -> u64 {
        self.dst_pub_key.mpn_account_index(log4_account_capacity)
    }
    pub fn verify_signature(&self) -> bool {
        jubjub::JubJub::<ZkMainHasher>::verify(&self.src_pub_key, self.hash(), &self.sig)
    }
    pub fn sign(&mut self, sk: &jubjub::PrivateKey) {
        self.sig = jubjub::JubJub::<ZkMainHasher>::sign(sk, self.hash());
    }
    pub fn hash(&self) -> ZkScalar {
        let dst_pub_decom = self.dst_pub_key.0.decompress();
        ZkMainHasher::hash(&[
            ZkScalar::from(self.nonce as u64),
            dst_pub_decom.0,
            dst_pub_decom.1,
            self.amount.token_id.into(),
            ZkScalar::from(self.amount.amount),
            self.fee.token_id.into(),
            ZkScalar::from(self.fee.amount),
        ])
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZkContract {
    pub initial_state: ZkCompressedState, // 32byte
    pub state_model: ZkStateModel,
    pub deposit_functions: Vec<ZkMultiInputVerifierKey>, // VK f(prev_state, deposit_txs (L1)) -> next_state
    pub withdraw_functions: Vec<ZkMultiInputVerifierKey>, // VK f(prev_state, withdraw_txs (L1)) -> next_state
    pub functions: Vec<ZkSingleInputVerifierKey>,         // Vec<VK> f(prev_state) -> next_state
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZkProof {
    Groth16(Box<groth16::Groth16Proof>),
    #[cfg(test)]
    Dummy(bool),
}

#[cfg(test)]
mod test;
