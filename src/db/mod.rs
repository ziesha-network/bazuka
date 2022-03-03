use db_key::Key;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum KvStoreError {
    #[error("kvstore failure")]
    Failure,
    #[error("kvstore data corrupted")]
    Corrupted(#[from] bincode::Error),
}

#[derive(Clone, Debug)]
pub struct StringKey(String);

impl StringKey {
    pub fn new(s: &str) -> StringKey {
        StringKey(s.to_string())
    }
}

#[derive(Clone, Debug)]
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

gen_try_into!(u32, u64, usize);

impl<T: serde::Serialize> From<T> for Blob {
    fn from(n: T) -> Self {
        Self(bincode::serialize(&n).unwrap())
    }
}

impl Key for StringKey {
    fn from_u8(key: &[u8]) -> StringKey {
        StringKey(std::str::from_utf8(key).unwrap().to_string())
    }

    fn as_slice<T, F: Fn(&[u8]) -> T>(&self, f: F) -> T {
        f(&self.0.as_bytes())
    }
}

pub enum WriteOp {
    Remove(StringKey),
    Put(StringKey, Blob),
}

pub trait KvStore {
    fn get(&self, k: StringKey) -> Result<Option<Blob>, KvStoreError>;
    fn del(&mut self, k: StringKey) -> Result<(), KvStoreError>;
    fn set(&mut self, k: StringKey, v: Blob) -> Result<(), KvStoreError>;
    fn batch(&mut self, ops: Vec<WriteOp>) -> Result<(), KvStoreError>;
}

pub struct RamMirrorKvStore<'a, K: KvStore> {
    store: &'a K,
    overwrite: HashMap<String, Option<Blob>>,
}
impl<'a, K: KvStore> RamMirrorKvStore<'a, K> {
    pub fn new(store: &'a K) -> Self {
        Self {
            store,
            overwrite: HashMap::new(),
        }
    }
}

impl<'a, K: KvStore> KvStore for RamMirrorKvStore<'a, K> {
    fn get(&self, k: StringKey) -> Result<Option<Blob>, KvStoreError> {
        if self.overwrite.contains_key(&k.0) {
            Ok(self.overwrite.get(&k.0).cloned().unwrap())
        } else {
            self.store.get(k)
        }
    }
    fn set(&mut self, k: StringKey, v: Blob) -> Result<(), KvStoreError> {
        self.overwrite.insert(k.0, Some(v));
        Ok(())
    }
    fn del(&mut self, k: StringKey) -> Result<(), KvStoreError> {
        self.overwrite.insert(k.0, None);
        Ok(())
    }
    fn batch(&mut self, ops: Vec<WriteOp>) -> Result<(), KvStoreError> {
        for op in ops.into_iter() {
            match op {
                WriteOp::Remove(k) => self.overwrite.insert(k.0, None),
                WriteOp::Put(k, v) => self.overwrite.insert(k.0, Some(v)),
            };
        }
        Ok(())
    }
}

mod ram;
pub use ram::*;

#[cfg(feature = "node")]
mod disk;
#[cfg(feature = "node")]
pub use disk::*;
