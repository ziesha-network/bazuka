use super::db::{KvStore, StringKey};
use super::primitives::{Address, Block, Money, Transaction};

pub trait Blockchain {
    fn get_balance(&self, addr: Address) -> Money;
    fn extend(&mut self, blocks: &Vec<Block>);
    fn get_height(&self) -> usize;
}

pub trait Identifiable {
    fn get_key(&self) -> StringKey;
}

impl Identifiable for Address {
    fn get_key(&self) -> StringKey {
        StringKey::new(&format!("addr_{:?}", self))
    }
}

pub struct KvStoreChain<K: KvStore> {
    database: K,
}

impl<K: KvStore> KvStoreChain<K> {
    pub fn new(kv_store: K) -> KvStoreChain<K> {
        KvStoreChain::<K> { database: kv_store }
    }
    fn apply_tx(tx: &Transaction) {}
}

impl<K: KvStore> Blockchain for KvStoreChain<K> {
    fn get_balance(&self, addr: Address) -> Money {
        match self.database.get(addr.get_key()).unwrap() {
            Some(b) => Money::from_le_bytes([b[0]]),
            None => 0,
        }
    }
    fn extend(&mut self, _blocks: &Vec<Block>) {
        unimplemented!();
    }
    fn get_height(&self) -> usize {
        0
    }
}
