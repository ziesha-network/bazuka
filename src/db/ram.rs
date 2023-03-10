use super::*;
use std::collections::BTreeMap;

pub struct RamKvStore(BTreeMap<StringKey, Blob>);
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
        Ok(self.0.get(&k).cloned())
    }
    fn update(&mut self, ops: &[WriteOp]) -> Result<(), KvStoreError> {
        for op in ops.iter() {
            match op {
                WriteOp::Remove(k) => self.0.remove(k),
                WriteOp::Put(k, v) => self.0.insert(k.clone(), v.clone()),
            };
        }
        Ok(())
    }
    fn pairs(&self, prefix: StringKey) -> Result<QueryResult, KvStoreError> {
        Ok(QueryResult::Ram {
            range: self
                .0
                .range((Bound::Included(prefix.clone()), Bound::Unbounded)),
            prefix,
        })
    }
}
