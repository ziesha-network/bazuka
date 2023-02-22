use super::*;
use std::collections::BTreeMap;

pub struct RamKvStore(BTreeMap<String, Blob>);
impl RamKvStore {
    pub fn new() -> RamKvStore {
        RamKvStore(BTreeMap::new())
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
    fn pairs(&self, prefix: StringKey) -> Result<Vec<(StringKey, Blob)>, KvStoreError> {
        Ok(self
            .0
            .clone()
            .into_iter()
            .filter_map(|(k, v)| {
                if k.starts_with(&prefix.0) {
                    Some((StringKey::new(&k), v))
                } else {
                    None
                }
            })
            .collect())
    }
}
