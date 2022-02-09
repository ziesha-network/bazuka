use std::fs;
use std::path::Path;
use std::sync::{Arc, RwLock};

use anyhow::{anyhow, Result};
use db_key::Key;
use leveldb::database::Database;
use leveldb::kv::KV;
use leveldb::options::{Options, ReadOptions, WriteOptions};

use crate::blockchain::header::{BackendHeader, HeaderMeta, HeaderMetaCache};
use crate::blockchain::utils::{read_meta, wrap_with_level_db, wrap_with_rocks_db};
use crate::blockchain::{utils, BackendDatabase, HeaderBackend};
use crate::core::block::BlockId;
use crate::core::header::Header;
use crate::core::U256;

pub struct BlockChainDB {
    database: Arc<dyn BackendDatabase>,
    meta: Arc<RwLock<ChainInfo>>,
    header_meta_cache: Arc<HeaderMetaCache>,
}

impl BlockChainDB {
    pub fn new(path: &Path) -> Result<BlockChainDB> {
        let mut options = rocksdb::Options::default();
        options.create_if_missing(true);
        let database = rocksdb::DB::open(&options, path)?;
        let database = wrap_with_rocks_db(database);
        let meta = read_meta(&*database)?;
        Ok(BlockChainDB {
            database,
            meta: Arc::new(RwLock::new(meta)),
            header_meta_cache: Arc::new(HeaderMetaCache::default()),
        })
    }
}

impl BackendHeader for BlockChainDB {
    fn get_header_meta(&self, hash: &[u8; 32]) -> Result<HeaderMeta> {
        self.header_meta_cache.get_header_meta(hash).map_or_else(
            || {
                self.header(BlockId::Hash(hash.clone()))?
                    .map(|h| {
                        let header_meta = HeaderMeta::from(&h);
                        self.header_meta_cache
                            .insert_header_meta(hash.clone(), header_meta.clone());
                        header_meta
                    })
                    .ok_or_else(|| anyhow!("header with hash {:?} was not found", hash))
            },
            Ok,
        )
    }

    fn insert_header_meta(&self, hash: [u8; 32], header: HeaderMeta) {
        self.header_meta_cache.insert_header_meta(hash, header);
    }

    fn remove_header(&self, hash: &[u8; 32]) {
        self.header_meta_cache.remove_header_meta(hash);
    }
}

impl HeaderBackend for BlockChainDB {
    fn header(&self, id: BlockId) -> Result<Option<Header>> {
        utils::get_header(&*self.database, &id)
    }

    fn info(&self) -> Result<ChainInfo> {
        let meta = self.meta.read()?;
        Ok(ChainInfo {
            best_hash: meta.best_hash.clone(),
            best_number: meta.best_number.clone(),
            genesis_hash: meta.genesis_hash.clone(),
            finalized_hash: meta.finalized_hash.clone(),
            finalized_number: meta.finalized_number.clone(),
            finalized_state: meta.finalized_state.clone(),
            number_leaves: meta.number_leaves,
            block_gap: meta.block_gap.clone(),
        })
    }

    fn status(&self, id: BlockId) -> Result<BlockStatus> {
        let exists = match id {
            BlockId::Hash(_) => self.header(id)?.is_some(),
            BlockId::Number(n) => n <= self.meta.read()?.best_number,
        };
        Ok(exists.into())
    }

    fn number(&self, hash: [u8; 32]) -> Result<Option<U256>> {
        Ok(Some(self.get_header_meta(&hash)?.number))
    }

    fn hash(&self, number: U256) -> Result<Option<[u8; 32]>> {
        let header = self.header(BlockId::Num(number))?;
        match header {
            None => Ok(None),
            Some(h) => Ok(Some(h.hash.clone())),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ChainInfo {
    pub best_hash: [u8; 32],
    pub best_number: U256,
    pub genesis_hash: [u8; 32],
    pub finalized_hash: [u8; 32],
    pub finalized_number: U256,
    pub finalized_state: Option<([u8; 32], U256)>,
    pub number_leaves: usize,
    pub block_gap: Option<(U256, U256)>,
}

impl Default for ChainInfo {
    fn default() -> Self {
        ChainInfo {
            best_hash: [0; 32],
            best_number: Default::default(),
            genesis_hash: [0; 32],
            finalized_hash: [0; 32],
            finalized_number: Default::default(),
            finalized_state: None,
            number_leaves: 0,
            block_gap: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockStatus {
    InChain,
    Unknown,
}

impl From<bool> for BlockStatus {
    fn from(b: bool) -> Self {
        match b {
            true => BlockStatus::InChain,
            false => BlockStatus::Unknown,
        }
    }
}
