use crate::blockchain::ZkBlockchainPatch;
use crate::core::{hash::Hash, Account, Block, ContractId, Hasher};
use crate::crypto::merkle::MerkleTree;
use crate::zk::{ZkCompressedState, ZkContract, ZkState};
use db_key::Key;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum KvStoreError {
    #[error("kvstore failure")]
    Failure,
    #[error("kvstore data corrupted: {0}")]
    Corrupted(#[from] bincode::Error),
    #[error("io error: {0}")]
    IO(#[from] std::io::Error),
    #[cfg(feature = "node")]
    #[error("leveldb error: {0}")]
    LevelDb(#[from] leveldb::error::Error),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, std::hash::Hash)]
pub struct StringKey(String);

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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Blob(Vec<u8>);

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
    Block,
    Vec<WriteOp>,
    MerkleTree<Hasher>,
    ZkContract,
    ZkCompressedState,
    HashMap<ContractId, ZkCompressedState>,
    ZkState,
    ZkBlockchainPatch
);
gen_from!(
    u32,
    u64,
    u128,
    usize,
    Account,
    &Block,
    Vec<WriteOp>,
    MerkleTree<Hasher>,
    ZkContract,
    ZkCompressedState,
    HashMap<ContractId, ZkCompressedState>,
    &ZkState,
    &ZkBlockchainPatch
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum WriteOp {
    Remove(StringKey),
    Put(StringKey, Blob),
}

pub trait KvStore {
    fn get(&self, k: StringKey) -> Result<Option<Blob>, KvStoreError>;
    fn update(&mut self, ops: &[WriteOp]) -> Result<(), KvStoreError>;
    fn pairs(&self) -> Result<HashMap<StringKey, Blob>, KvStoreError>;
    fn checksum<H: Hash>(&self) -> Result<H::Output, KvStoreError> {
        let mut kvs: Vec<_> = self.pairs()?.into_iter().collect();
        kvs.sort_by_key(|(k, _)| k.clone());
        Ok(H::hash(&bincode::serialize(&kvs).unwrap()))
    }
    fn rollback_of(&self, ops: &[WriteOp]) -> Result<Vec<WriteOp>, KvStoreError> {
        let mut rollback = Vec::new();
        for op in ops.iter() {
            let key = match op {
                WriteOp::Put(k, _) => k,
                WriteOp::Remove(k) => k,
            }
            .clone();
            rollback.push(match self.get(key.clone())? {
                Some(b) => WriteOp::Put(key, b.clone()),
                None => WriteOp::Remove(key),
            })
        }
        Ok(rollback)
    }
}

pub struct RamMirrorKvStore<'a, K: KvStore> {
    store: &'a K,
    overwrite: HashMap<StringKey, Option<Blob>>,
}
impl<'a, K: KvStore> RamMirrorKvStore<'a, K> {
    pub fn new(store: &'a K) -> Self {
        Self {
            store,
            overwrite: HashMap::new(),
        }
    }
    pub fn to_ops(self) -> Vec<WriteOp> {
        self.overwrite
            .into_iter()
            .map(|(k, v)| match v {
                Some(b) => WriteOp::Put(k, b),
                None => WriteOp::Remove(k),
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
    fn pairs(&self) -> Result<HashMap<StringKey, Blob>, KvStoreError> {
        let mut res = self.store.pairs()?;
        for (k, v) in self.overwrite.iter() {
            if let Some(v) = v {
                res.insert(k.clone(), v.clone());
            } else {
                res.remove(k);
            }
        }
        Ok(res)
    }
}

mod ram;
pub use ram::*;

#[cfg(feature = "node")]
mod disk;
#[cfg(feature = "node")]
pub use disk::*;

#[cfg(test)]
mod test;
