use super::*;
use leveldb::batch::Batch;
use leveldb::database::batch::Writebatch;
use leveldb::database::cache::Cache;
use leveldb::database::{
    snapshots::{Snapshot, Snapshots},
    Database,
};
use leveldb::iterator::Iterable;
use leveldb::iterator::LevelDBIterator;
use leveldb::kv::KV;
use leveldb::options::{Options, ReadOptions, WriteOptions};
use std::fs;
use std::path::{Path, PathBuf};
use tempdir::TempDir;

pub struct ReadOnlyLevelDbKvStore {
    mirror_path: PathBuf,
    db: Option<Database<StringKey>>,
}
pub struct LevelDbSnapshot<'a>(Snapshot<'a, StringKey>);
impl ReadOnlyLevelDbKvStore {
    pub fn read_only(
        path: &Path,
        cache_size: usize,
    ) -> Result<ReadOnlyLevelDbKvStore, KvStoreError> {
        let link_dir = TempDir::new("bazuka_mirror")?.into_path();
        for p in std::fs::read_dir(path)? {
            let p = p?;
            if !p.file_name().eq_ignore_ascii_case("lock") {
                std::os::unix::fs::symlink(p.path(), link_dir.join(p.file_name()))?;
            }
        }
        let mut options = Options::new();
        options.cache = Some(Cache::new(cache_size));
        Ok(ReadOnlyLevelDbKvStore {
            db: Some(Database::open(&link_dir, options)?),
            mirror_path: link_dir,
        })
    }
    pub fn snapshot(&self) -> LevelDbSnapshot {
        LevelDbSnapshot(self.db.as_ref().unwrap().snapshot())
    }
}

impl Drop for ReadOnlyLevelDbKvStore {
    fn drop(&mut self) {
        self.db = None;
        let _ = std::fs::remove_dir_all(&self.mirror_path);
    }
}

pub struct LevelDbKvStore(Database<StringKey>);
impl LevelDbKvStore {
    pub fn new(path: &Path, cache_size: usize) -> Result<LevelDbKvStore, KvStoreError> {
        fs::create_dir_all(&path)?;
        let mut options = Options::new();
        options.create_if_missing = true;
        options.cache = Some(Cache::new(cache_size));
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
    fn pairs(&self, prefix: StringKey) -> Result<Vec<(StringKey, Blob)>, KvStoreError> {
        let it = self.0.iter(ReadOptions::new());
        it.seek(&prefix);
        Ok(it
            .collect::<Vec<_>>()
            .into_iter()
            .take_while(|(k, _)| k.0.starts_with(&prefix.0))
            .map(|(k, v)| (k, Blob(v)))
            .collect())
    }
}

impl<'a> KvStore for LevelDbSnapshot<'a> {
    fn get(&self, k: StringKey) -> Result<Option<Blob>, KvStoreError> {
        let read_opts = ReadOptions::new();
        match self.0.get(read_opts, k) {
            Ok(v) => Ok(v.map(Blob)),
            Err(_) => Err(KvStoreError::Failure),
        }
    }
    fn update(&mut self, _: &[WriteOp]) -> Result<(), KvStoreError> {
        panic!("Cannot update!");
    }
    fn pairs(&self, prefix: StringKey) -> Result<Vec<(StringKey, Blob)>, KvStoreError> {
        let it = self.0.iter(ReadOptions::new());
        it.seek(&prefix);
        let mut result = Vec::new();
        for (k, v) in it {
            if k.0.starts_with(&prefix.0) {
                result.push((k, Blob(v)));
            } else {
                break;
            }
        }
        Ok(result)
    }
}
