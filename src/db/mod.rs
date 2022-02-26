use db_key::Key;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum KvStoreError {
    #[error("kvstore failure")]
    Failure,
    #[error("kvstore data corrupted")]
    Corrupted,
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

impl Blob {
    pub fn as_u32(&self) -> Result<u32, KvStoreError> {
        Ok(0)
    }
    pub fn as_u64(&self) -> Result<u64, KvStoreError> {
        Ok(0)
    }
    pub fn as_usize(&self) -> Result<usize, KvStoreError> {
        Ok(0)
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

mod ram;
pub use ram::*;

#[cfg(feature = "node")]
mod disk;
#[cfg(feature = "node")]
pub use disk::*;
