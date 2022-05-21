use super::*;
use leveldb::batch::Batch;
use leveldb::database::batch::Writebatch;
use leveldb::database::Database;
use leveldb::iterator::Iterable;
use leveldb::kv::KV;
use leveldb::options::{Options, ReadOptions, WriteOptions};
use std::fs;
use std::path::Path;

pub struct LevelDbKvStore(Database<StringKey>);
impl LevelDbKvStore {
    pub fn new(path: &Path) -> Result<LevelDbKvStore, KvStoreError> {
        fs::create_dir_all(&path)?;
        let mut options = Options::new();
        options.create_if_missing = true;
        Ok(LevelDbKvStore(Database::open(path, options)?))
    }
}

impl KvStore for LevelDbKvStore {
    fn get(&self, k: StringKey) -> Result<Option<Blob>, KvStoreError> {
        let read_opts = ReadOptions::new();
        match self.0.get(read_opts, k) {
            Ok(v) => Ok(v.map(Blob)),
            Err(_) => Err(KvStoreError::Failure),
        }
    }
    fn update(&mut self, ops: &[WriteOp]) -> Result<(), KvStoreError> {
        let write_opts = WriteOptions::new();
        let mut batch = Writebatch::new();
        for op in ops.iter() {
            match op {
                WriteOp::Remove(k) => batch.delete(k.clone()),
                WriteOp::Put(k, v) => batch.put(k.clone(), &v.0),
            }
        }
        match self.0.write(write_opts, &batch) {
            Ok(_) => Ok(()),
            Err(_) => Err(KvStoreError::Failure),
        }
    }
    fn checksum<H: Hash>(&self) -> Result<H::Output, KvStoreError> {
        let mut kvs: Vec<_> = self.0.iter(ReadOptions::new()).collect();
        kvs.sort_by_key(|(k, _)| k.0.clone());
        Ok(H::hash(&bincode::serialize(&kvs).unwrap()))
    }
}
