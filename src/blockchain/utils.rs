use anyhow::{Error, Result};
use byteorder::WriteBytesExt;
use db_key::Key;
use ff::derive::bitvec::view::AsBits;
use leveldb::kv::KV;
use leveldb::options::ReadOptions;
use log::debug;
use num_traits::Zero;
use rocksdb::{ColumnFamily, DBWithThreadMode, SingleThreaded};

use primitive_types::U256;

use crate::blockchain::backend::ChainInfo;
use crate::blockchain::DataType::Meta;
use crate::blockchain::{BackendDatabase, DataType, DataTypes, HeaderBackend};
use crate::core::block::BlockId;
use crate::core::hash;
use crate::core::header::Header;

struct LevelDbWrapper<K: Key, D: KV<K> + 'static>(D);

impl<K: Key, D: KV<K>> BackendDatabase for LevelDbWrapper<K, D> {
    fn get(&self, _: DataType, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let output = self.0.get(ReadOptions::new(), key)?;
        Ok(output)
    }

    fn commit() -> Result<()> {
        todo!()
    }
}

struct RocksDBWrapper(rocksdb::DBWithThreadMode<SingleThreaded>);

impl BackendDatabase for RocksDBWrapper {
    fn get(&self, typ: DataType, key: &[u8]) -> Result<Option<Vec<u8>>> {
        Ok(match self.0.cf_handle(DataTypes.get(&typ).unwrap()) {
            None => self.0.get(key)?,
            Some(col) => self.0.get_cf(col, key)?,
        })
    }

    fn commit() -> Result<()> {
        todo!()
    }
}

pub fn wrap_with_rocks_db(
    db: DBWithThreadMode<SingleThreaded>,
) -> std::sync::Arc<dyn BackendDatabase> {
    std::sync::Arc::new(RocksDBWrapper(db))
}

pub fn wrap_with_level_db<K, D>(db: D) -> std::sync::Arc<dyn BackendDatabase>
where
    K: Key,
    D: KV<K> + 'static,
{
    std::sync::Arc::new(LevelDbWrapper(db))
}

pub mod meta_keys {
    pub const TYPE: &[u8; 4] = b"type";
    pub const BEST_BLOCK: &[u8; 4] = b"best";
    pub const FINALIZED_BLOCK: &[u8; 5] = b"final";
    pub const FINALIZED_STATE: &[u8; 6] = b"fstate";
    pub const BLOCK_GAP: &[u8; 3] = b"gap";
    pub const GENESIS_HASH: &[u8; 4] = b"gene";
}

pub fn read_meta(db: &dyn BackendDatabase) -> Result<ChainInfo> {
    let genesis_hash: [u8; 32] = match read_genesis_hash(db)? {
        Some(h) => h.try_into()?,
        None => return Ok(Default::default()),
    };
    let with_meta_key = |key| -> Result<_> {
        if let Some(data) = db
            .get(DataType::Meta, key)?
            .and_then(|hash| db.get(typ, hash.as_slice())?)
        {
            let h: Header = serde_json::from_slice(data.as_slice())?;
            debug!(target: "backend", "fetched hash {:?} and number {} from database", h.hash, h.number);
            Ok((h.hash, h.number))
        } else {
            Ok(([0; 32], Zero::zero()))
        }
    };
    let (best_hash, best_number) = with_meta_key(meta_keys::BEST_BLOCK)?;
    let (finalized_hash, finalized_number) = with_meta_key(meta_keys::FINALIZED_BLOCK)?;
    let (finalized_state_hash, finalized_state_number) = with_meta_key(meta_keys::FINALIZED_STATE)?;
    let block_gap = if let Some(data) = db.get(DataType::Meta, meta_keys::BLOCK_GAP)? {
        let u256_len = U256::one().0.len();
        assert_eq!(
            data.len(),
            u256_len * 2,
            "META: block gap must has length of {}, but it is {} actually",
            u256_len * 2,
            data.len()
        );
        debug!(target: "backend", "fetch block gap {:?}", data);
        let left = U256::decode_all(&data[..u256_len])?;
        let right = U256::decode_all(&data[u256_len..u256_len * 2])?;
        Some((left, right))
    } else {
        None
    };
    return Ok(ChainInfo {
        best_hash,
        best_number,
        genesis_hash,
        finalized_hash,
        finalized_number,
        finalized_state: if finalized_state_hash != hash::Zero {
            Some((finalized_state_hash, finalized_state_number))
        } else {
            None
        },
        number_leaves: 0,
        block_gap,
    });
}

pub fn read_genesis_hash(db: &dyn BackendDatabase) -> Result<Option<Vec<u8>>> {
    db.get(DataType::Meta, meta_keys::GENESIS_HASH)
}

pub fn index_lookup(db: &dyn BackendDatabase, id: &BlockId) -> Result<Option<Vec<u8>>> {
    match id {
        BlockId::Hash(hash) => db.get(DataType::HashIdx, hash),
        BlockId::Num(num) => {
            use byteorder::{LittleEndian, WriteBytesExt};
            let mut v = vec![0; 32];
            v.write_u64::<LittleEndian>(num.0[0]);
            v.write_u64::<LittleEndian>(num.0[1]);
            v.write_u64::<LittleEndian>(num.0[2]);
            v.write_u64::<LittleEndian>(num.0[3]);
            db.get(DataType::NumIdx, v.as_slice())
        }
    }
}

pub fn get_header(db: &dyn BackendDatabase, id: &BlockId) -> Result<Option<Header>> {
    match index_lookup(db, id).and_then(|key| match key {
        None => Ok(None),
        Some(k) => db.get(DataType::Header, k.as_slice()),
    })? {
        None => Ok(None),
        Some(data) => Ok(Some(serde_json::from_slice::<Header>(data.as_slice())?)),
    }
}
