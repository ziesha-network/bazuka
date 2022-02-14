use db_key::Key;
use leveldb::batch::Batch;
use leveldb::database::batch::Writebatch;
use leveldb::database::Database;
use leveldb::kv::KV;
use leveldb::options::{Options, ReadOptions, WriteOptions};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct StringKey(String);

impl StringKey {
    pub fn new(s: &str) -> StringKey {
        StringKey(s.to_string())
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

#[derive(Debug)]
pub enum KvStoreError {
    Failure,
}

pub enum WriteOp {
    Remove(StringKey),
    Put(StringKey, Vec<u8>),
}

pub trait KvStore {
    fn get(&self, k: StringKey) -> Result<Option<Vec<u8>>, KvStoreError>;
    fn del(&mut self, k: StringKey) -> Result<(), KvStoreError>;
    fn set(&mut self, k: StringKey, v: Vec<u8>) -> Result<(), KvStoreError>;
    fn batch(&mut self, ops: Vec<WriteOp>) -> Result<(), KvStoreError>;
}

pub struct LevelDbKvStore(Database<StringKey>);
impl LevelDbKvStore {
    pub fn new(path: &Path) -> LevelDbKvStore {
        fs::create_dir_all(&path).unwrap();
        let mut options = Options::new();
        options.create_if_missing = true;
        LevelDbKvStore(Database::open(&path, options).unwrap())
    }
}

impl KvStore for LevelDbKvStore {
    fn get(&self, k: StringKey) -> Result<Option<Vec<u8>>, KvStoreError> {
        let read_opts = ReadOptions::new();
        match self.0.get(read_opts, k) {
            Ok(v) => Ok(v),
            Err(_) => Err(KvStoreError::Failure),
        }
    }
    fn set(&mut self, k: StringKey, v: Vec<u8>) -> Result<(), KvStoreError> {
        let write_opts = WriteOptions::new();
        match self.0.put(write_opts, k, &v) {
            Ok(_) => Ok(()),
            Err(_) => Err(KvStoreError::Failure),
        }
    }
    fn del(&mut self, k: StringKey) -> Result<(), KvStoreError> {
        let write_opts = WriteOptions::new();
        match self.0.delete(write_opts, k) {
            Ok(_) => Ok(()),
            Err(_) => Err(KvStoreError::Failure),
        }
    }
    fn batch(&mut self, ops: Vec<WriteOp>) -> Result<(), KvStoreError> {
        let write_opts = WriteOptions::new();
        let mut batch = Writebatch::new();
        for op in ops.into_iter() {
            match op {
                WriteOp::Remove(k) => batch.delete(k),
                WriteOp::Put(k, v) => batch.put(k, &v),
            }
        }
        match self.0.write(write_opts, &batch) {
            Ok(_) => Ok(()),
            Err(_) => Err(KvStoreError::Failure),
        }
    }
}

pub struct RamKvStore(HashMap<String, Vec<u8>>);
impl RamKvStore {
    pub fn new() -> RamKvStore {
        RamKvStore(HashMap::new())
    }
}

impl KvStore for RamKvStore {
    fn get(&self, k: StringKey) -> Result<Option<Vec<u8>>, KvStoreError> {
        Ok(self.0.get(&k.0).cloned())
    }
    fn set(&mut self, k: StringKey, v: Vec<u8>) -> Result<(), KvStoreError> {
        self.0.insert(k.0, v);
        Ok(())
    }
    fn del(&mut self, k: StringKey) -> Result<(), KvStoreError> {
        self.0.remove(&k.0);
        Ok(())
    }
    fn batch(&mut self, ops: Vec<WriteOp>) -> Result<(), KvStoreError> {
        for op in ops.into_iter() {
            match op {
                WriteOp::Remove(k) => self.0.remove(&k.0),
                WriteOp::Put(k, v) => self.0.insert(k.0, v),
            };
        }
        Ok(())
    }
}
