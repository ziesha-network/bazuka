use super::*;
use std::collections::HashMap;

pub struct RamKvStore(HashMap<String, Blob>);
impl RamKvStore {
    pub fn new() -> RamKvStore {
        RamKvStore(HashMap::new())
    }
}

impl KvStore for RamKvStore {
    fn get(&self, k: StringKey) -> Result<Option<Blob>, KvStoreError> {
        Ok(self.0.get(&k.0).cloned())
    }
    fn set(&mut self, k: StringKey, v: Blob) -> Result<(), KvStoreError> {
        self.0.insert(k.0, v);
        Ok(())
    }
    fn del(&mut self, k: StringKey) -> Result<(), KvStoreError> {
        self.0.remove(&k.0);
        Ok(())
    }
    fn batch(&mut self, ops: &Vec<WriteOp>) -> Result<(), KvStoreError> {
        for op in ops.iter() {
            match op {
                WriteOp::Remove(k) => self.0.remove(&k.0),
                WriteOp::Put(k, v) => self.0.insert(k.0.clone(), v.clone()),
            };
        }
        Ok(())
    }
}
