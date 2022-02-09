use super::*;
use leveldb::batch::Batch;
use leveldb::database::batch::Writebatch;
use leveldb::database::Database;
use leveldb::kv::KV;
use leveldb::options::{Options, ReadOptions, WriteOptions};
use std::fs;
use std::path::Path;

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
    fn get(&self, k: StringKey) -> Result<Option<Blob>, KvStoreError> {
        let read_opts = ReadOptions::new();
        match self.0.get(read_opts, k) {
            Ok(v) => Ok(v.map(|v| Blob(v))),
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
    fn set(&mut self, k: StringKey, v: Blob) -> Result<(), KvStoreError> {
        let write_opts = WriteOptions::new();
        match self.0.put(write_opts, k, &v.0) {
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
                WriteOp::Put(k, v) => batch.put(k, &v.0),
            }
        }
        match self.0.write(write_opts, &batch) {
            Ok(_) => Ok(()),
            Err(_) => Err(KvStoreError::Failure),
        }
    }
}
