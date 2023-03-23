pub mod keys;

use crate::blockchain::{ZkBlockchainPatch, ZkCompressedStateChange};
use crate::core::{
    hash::Hash, Account, Amount, Block, ContractAccount, ContractId, Delegate, Hasher, Header,
    Staker, Token,
};
use crate::crypto::merkle::MerkleTree;
use crate::zk::{
    ZkCompressedState, ZkContract, ZkDataPairs, ZkDeltaPairs, ZkScalar, ZkState, ZkStateModel,
};
use db_key::Key;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::ops::Bound;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum KvStoreError {
    #[error("kvstore failure")]
    Failure,
    #[error("kvstore data corrupted: {0}")]
    Corrupted(#[from] bincode::Error),
    #[error("io error: {0}")]
    IO(#[from] std::io::Error),
    #[cfg(feature = "db")]
    #[error("leveldb error: {0}")]
    LevelDb(#[from] leveldb::error::Error),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, std::hash::Hash)]
pub struct StringKey(pub String);

impl std::fmt::Display for StringKey {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PartialOrd for StringKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for StringKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl StringKey {
    pub fn new(s: &str) -> StringKey {
        StringKey(s.to_string())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Blob(pub Vec<u8>);

impl std::fmt::Display for Blob {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", hex::encode(&self.0))
    }
}

macro_rules! gen_try_into {
    ( $( $x:ty ),* ) => {
        $(
            impl TryInto<$x> for Blob {
                type Error = KvStoreError;
                fn try_into(self) -> Result<$x, Self::Error> {
                    Ok(bincode::deserialize(&self.0)?)
                }
            }
        )*
    };
}

macro_rules! gen_from {
    ( $( $x:ty ),* ) => {
        $(
            impl From<$x> for Blob {
                fn from(n: $x) -> Self {
                    Self(bincode::serialize(&n).unwrap())
                }
            }
        )*
    };
}

gen_try_into!(
    u32,
    u64,
    u128,
    usize,
    Account,
    Delegate,
    Staker,
    ContractAccount,
    Header,
    Block,
    Vec<WriteOp>,
    MerkleTree<Hasher>,
    ZkContract,
    ZkCompressedState,
    Vec<ContractId>,
    HashMap<ContractId, ContractAccount>,
    HashMap<ContractId, ZkCompressedStateChange>,
    ZkState,
    ZkBlockchainPatch,
    ZkStateModel,
    ZkScalar,
    ZkDataPairs,
    ZkDeltaPairs,
    Token,
    Amount,
    ()
);
gen_from!(
    u32,
    u64,
    u128,
    usize,
    Account,
    Delegate,
    Staker,
    ContractAccount,
    Header,
    &Block,
    Vec<WriteOp>,
    MerkleTree<Hasher>,
    ZkContract,
    ZkCompressedState,
    Vec<ContractId>,
    HashMap<ContractId, ContractAccount>,
    HashMap<ContractId, ZkCompressedStateChange>,
    &ZkState,
    &ZkBlockchainPatch,
    ZkStateModel,
    ZkScalar,
    &ZkDataPairs,
    &ZkDeltaPairs,
    &Token,
    Amount,
    ()
);

impl Key for StringKey {
    fn from_u8(key: &[u8]) -> StringKey {
        StringKey(std::str::from_utf8(key).unwrap().to_string())
    }

    fn as_slice<T, F: Fn(&[u8]) -> T>(&self, f: F) -> T {
        f(self.0.as_bytes())
    }
}

impl From<String> for StringKey {
    fn from(s: String) -> Self {
        Self::new(&s)
    }
}
impl From<&str> for StringKey {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum WriteOp {
    Remove(StringKey),
    Put(StringKey, Blob),
}

pub enum QueryResult<'a> {
    Precalculated(Vec<(StringKey, Blob)>),
    LevelDb {
        prefix: StringKey,
        db: leveldb::iterator::Iterator<'a, StringKey>,
    },
    Ram {
        range: std::collections::btree_map::Range<'a, StringKey, Blob>,
        prefix: StringKey,
    },
    Mirror {
        actual: Box<QueryResult<'a>>,
        overwrite: std::collections::btree_map::Range<'a, StringKey, Option<Blob>>,
        prefix: StringKey,
    },
}
pub struct MirroredQueryResultIterator<'a> {
    prefix: StringKey,
    actual_iter: Box<dyn std::iter::Iterator<Item = (StringKey, Blob)> + 'a>,
    curr_actual: Option<(StringKey, Blob)>,
    overwrite_iter: std::vec::IntoIter<(StringKey, Option<Blob>)>,
    curr_overwrite: Option<(StringKey, Option<Blob>)>,
}

enum MirroredQueryResultIteratorElement {
    Skipped,
    Item((StringKey, Blob)),
}

impl<'a> MirroredQueryResultIterator<'a> {
    fn vnext(&mut self) -> Option<MirroredQueryResultIteratorElement> {
        if self.curr_actual.is_none() {
            self.curr_actual = self.actual_iter.next();
        }
        if self.curr_overwrite.is_none() {
            self.curr_overwrite = self.overwrite_iter.next();
        }
        match (self.curr_actual.clone(), self.curr_overwrite.clone()) {
            (Some(curr_actual), Some(curr_overwrite)) => {
                if curr_actual.0 < curr_overwrite.0 {
                    self.curr_actual = None;
                    Some(MirroredQueryResultIteratorElement::Item(curr_actual))
                } else {
                    self.curr_overwrite = None;
                    if curr_actual.0 == curr_overwrite.0 {
                        self.curr_actual = None;
                    }
                    if let Some(v) = curr_overwrite.1 {
                        Some(MirroredQueryResultIteratorElement::Item((
                            curr_overwrite.0,
                            v,
                        )))
                    } else {
                        Some(MirroredQueryResultIteratorElement::Skipped)
                    }
                }
            }
            (Some(curr_actual), None) => {
                self.curr_actual = None;
                Some(MirroredQueryResultIteratorElement::Item(curr_actual))
            }
            (None, Some(curr_overwrite)) => {
                self.curr_overwrite = None;
                Some(if let (k, Some(v)) = curr_overwrite.clone() {
                    MirroredQueryResultIteratorElement::Item((k, v))
                } else {
                    MirroredQueryResultIteratorElement::Skipped
                })
            }
            (None, None) => None,
        }
    }
}

impl<'a> Iterator for MirroredQueryResultIterator<'a> {
    type Item = (StringKey, Blob);
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(elem) = self.vnext() {
            if let MirroredQueryResultIteratorElement::Item((k, v)) = elem {
                return if k.0.starts_with(&self.prefix.0) {
                    Some((k, v))
                } else {
                    None
                };
            }
        }
        None
    }
}
impl<'a> QueryResult<'a> {
    pub fn into_iter(self) -> Box<dyn Iterator<Item = (StringKey, Blob)> + 'a> {
        match self {
            QueryResult::Precalculated(v) => Box::new(v.into_iter()),
            QueryResult::LevelDb { prefix, db } => Box::new(
                db.map(|(k, v)| (k, Blob(v)))
                    .take_while(move |(k, _)| k.0.starts_with(&prefix.0)),
            ),
            QueryResult::Ram { range, prefix } => Box::new(
                range
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .take_while(move |(k, _)| k.0.starts_with(&prefix.0)),
            ),
            QueryResult::Mirror {
                actual,
                overwrite,
                prefix,
            } => Box::new(MirroredQueryResultIterator::<'a> {
                actual_iter: actual.into_iter(),
                overwrite_iter: overwrite
                    .take_while(|(k, _)| k.0.starts_with(&prefix.0))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect::<Vec<_>>()
                    .into_iter(),
                curr_actual: None,
                curr_overwrite: None,
                prefix,
            }),
        }
    }
    pub fn checksum<H: Hash>(self) -> Result<H::Output, KvStoreError> {
        let mut kvs: Vec<_> = self.into_iter().collect();
        kvs.sort_by_key(|(k, _)| k.clone());
        Ok(H::hash(&bincode::serialize(&kvs).unwrap()))
    }
}

pub trait KvStore {
    fn get(&self, k: StringKey) -> Result<Option<Blob>, KvStoreError>;
    fn update(&mut self, ops: &[WriteOp]) -> Result<(), KvStoreError>;
    fn pairs(&self, prefix: StringKey) -> Result<QueryResult, KvStoreError>;
    fn mirror(&self) -> RamMirrorKvStore<'_, Self>
    where
        Self: Sized,
    {
        RamMirrorKvStore::new(self)
    }
}

pub struct RamMirrorKvStore<'a, K: KvStore> {
    store: &'a K,
    overwrite: BTreeMap<StringKey, Option<Blob>>,
}
impl<'a, K: KvStore> RamMirrorKvStore<'a, K> {
    pub fn new(store: &'a K) -> Self {
        Self {
            store,
            overwrite: BTreeMap::new(),
        }
    }
    pub fn rollback(&self) -> Result<Vec<WriteOp>, KvStoreError> {
        self.overwrite
            .iter()
            .map(|(k, _)| {
                self.store.get(k.clone()).map(|v| match v {
                    Some(v) => WriteOp::Put(k.clone(), v),
                    None => WriteOp::Remove(k.clone()),
                })
            })
            .collect()
    }
    pub fn to_ops(&self) -> Vec<WriteOp> {
        self.overwrite
            .iter()
            .map(|(k, v)| match v {
                Some(b) => WriteOp::Put(k.clone(), b.clone()),
                None => WriteOp::Remove(k.clone()),
            })
            .collect()
    }
}

impl<'a, K: KvStore> KvStore for RamMirrorKvStore<'a, K> {
    fn get(&self, k: StringKey) -> Result<Option<Blob>, KvStoreError> {
        if self.overwrite.contains_key(&k) {
            Ok(self.overwrite.get(&k).cloned().unwrap())
        } else {
            self.store.get(k)
        }
    }
    fn update(&mut self, ops: &[WriteOp]) -> Result<(), KvStoreError> {
        for op in ops.iter() {
            match op {
                WriteOp::Remove(k) => self.overwrite.insert(k.clone(), None),
                WriteOp::Put(k, v) => self.overwrite.insert(k.clone(), Some(v.clone())),
            };
        }
        Ok(())
    }
    fn pairs(&self, prefix: StringKey) -> Result<QueryResult, KvStoreError> {
        Ok(QueryResult::Mirror {
            actual: Box::new(self.store.pairs(prefix.clone())?),
            overwrite: self
                .overwrite
                .range((Bound::Included(prefix.clone()), Bound::Unbounded)),
            prefix,
        })
    }
}

mod ram;
pub use ram::*;

#[cfg(feature = "db")]
mod disk;
#[cfg(feature = "db")]
pub use disk::*;

#[cfg(test)]
mod test;
