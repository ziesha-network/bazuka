use super::*;
use std::collections::HashMap;

pub struct RamKvStore(HashMap<String, Blob>);
impl RamKvStore {
    pub fn new() -> RamKvStore {
        RamKvStore(HashMap::new())
    }
}

impl Default for RamKvStore {
    fn default() -> Self {
        Self::new()
    }
}

impl KvStore for RamKvStore {
    fn get(&self, k: StringKey) -> Result<Option<Blob>, KvStoreError> {
        Ok(self.0.get(&k.0).cloned())
    }
    fn update(&mut self, ops: &[WriteOp]) -> Result<(), KvStoreError> {
        for op in ops.iter() {
            match op {
                WriteOp::Remove(k) => self.0.remove(&k.0),
                WriteOp::Put(k, v) => self.0.insert(k.0.clone(), v.clone()),
            };
        }
        Ok(())
    }
    fn checksum<H: Hash>(&self) -> Result<H::Output, KvStoreError> {
        let mut kvs: Vec<_> = self.0.clone().into_iter().collect();
        kvs.sort_by_key(|(k, _)| k.clone());
        Ok(H::hash(&bincode::serialize(&kvs).unwrap()))
    }
}
