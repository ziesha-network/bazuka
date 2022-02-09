use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::Result;

use crate::blockchain::backend::{BlockStatus, ChainInfo};
use crate::blockchain::header::HeaderMeta;
use crate::core::block::BlockId;
use crate::core::header::Header;
use crate::core::{Address, Block, Money, Transaction, U256};
use crate::db::{KvStore, KvStoreError, StringKey, WriteOp};

mod backend;
mod error;
mod header;
mod utils;

lazy_static! {
    static ref DataTypes: HashMap<DataType, &'static str> = {
        let mut m = HashMap::new();
        m.insert(DataType::Meta, DataType::Meta.into());
        m.insert(DataType::Header, DataType::Header.into());
        m.insert(DataType::Body, DataType::Body.into());
        // index
        m.insert(DataType::HashIdx, DataType::HashIdx.into());
        m.insert(DataType::NumIdx, DataType::NumIdx.into());
        m
    };
}

pub trait Blockchain {
    fn get_balance(&self, addr: Address) -> Result<Money>;
    fn extend(&mut self, blocks: &Vec<Block>) -> Result<()>;
    fn get_height(&self) -> Result<usize>;
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
    fn apply_tx(&self, _tx: &Transaction) -> Vec<WriteOp> {
        unimplemented!();
    }
    fn apply_block(&self, block: &Block) -> Vec<WriteOp> {
        let mut changes = Vec::new();
        for tx in block.body.iter() {
            changes.extend(self.apply_tx(tx))
        }
        changes
    }
}

impl<K: KvStore> Blockchain for KvStoreChain<K> {
    fn get_balance(&self, addr: Address) -> Result<Money> {
        Ok(match self.database.get(addr.get_key())? {
            Some(b) => b.as_u32()?,
            None => 0,
        })
    }
    fn extend(&mut self, blocks: &Vec<Block>) -> Result<()> {
        for block in blocks.iter() {
            let delta = self.apply_block(block);
            self.database.batch(delta)?;
        }
        unimplemented!();
    }
    fn get_height(&self) -> Result<usize> {
        Ok(match self.database.get(StringKey::new("height"))? {
            Some(b) => b.as_usize()?,
            None => 0,
        })
    }
}

pub trait BackendDatabase: Send + Sync {
    fn get(&self, typ: DataType, key: &[u8]) -> Result<Option<Vec<u8>>>;
    fn commit() -> Result<()>;
}

pub trait HeaderBackend: Sync + Send {
    fn header(&self, id: BlockId) -> Result<Option<Header>>;
    fn info(&self) -> Result<ChainInfo>;
    fn status(&self, id: BlockId) -> Result<BlockStatus>;
    fn number(&self, hash: [u8; 32]) -> Result<Option<U256>>;
    fn hash(&self, number: U256) -> Result<Option<[u8; 32]>>;
    fn arbitrary_hash_of_block(&self, id: &BlockId) -> Result<Option<[u8; 32]>> {
        match *id {
            BlockId::Hash(h) => Ok(Some(h)),
            BlockId::Num(num) => self.hash(n),
        }
    }

    fn arbitrary_id_of_block(&self, id: &BlockId) -> Result<Option<U256>> {
        match *id {
            BlockId::Hash(h) => self.number(h),
            BlockId::Num(num) => Ok(Some(num)),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DataType {
    Meta,
    Header,
    Body,
    HashIdx,
    NumIdx,
}

impl<'a> From<&DataType> for &'a str {
    fn from(typ: &DataType) -> Self {
        match *typ {
            DataType::Meta => "meta",
            DataType::Header => "header",
            DataType::Body => "body",
            DataType::HashIdx => "hashidx",
            DataType::NumIdx => "numidx",
        }
    }
}
