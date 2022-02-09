use super::*;
use std::collections::HashMap;

pub struct RamKvStore(HashMap<String, Vec<u8>>);
impl RamKvStore {
    pub fn new() -> RamKvStore {
        RamKvStore(HashMap::new())
    }
}

impl KvStore for RamKvStore {
    fn get(&self, k: StringKey) -> Result<Option<Blob>, KvStoreError> {
        Ok(self.0.get(&k.0).cloned().map(|v| Blob(v)))
    }
    fn del(&mut self, k: StringKey) -> Result<(), KvStoreError> {
        self.0.remove(&k.0);
        Ok(())
    }
    fn set(&mut self, k: StringKey, v: Blob) -> Result<(), KvStoreError> {
        self.0.insert(k.0, v.0);
        Ok(())
    }
    fn batch(&mut self, ops: Vec<WriteOp>) -> Result<(), KvStoreError> {
        for op in ops.into_iter() {
            match op {
                WriteOp::Remove(k) => self.0.remove(&k.0),
                WriteOp::Put(k, v) => self.0.insert(k.0, v.0),
            };
        }
        Ok(())
    }
}
