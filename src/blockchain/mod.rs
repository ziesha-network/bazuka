use crate::config::genesis;
use crate::core::{Address, Block, Money, Transaction};
use crate::db::{KvStore, KvStoreError, StringKey, WriteOp};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BlockchainError {
    #[error("kvstore error happened")]
    KvStoreError(#[from] KvStoreError),
    #[error("transaction signature is invalid")]
    SignatureError,
    #[error("balance insufficient")]
    BalanceInsufficient
}

pub trait Blockchain {
    fn get_balance(&self, addr: Address) -> Result<Money, BlockchainError>;
    fn extend(&mut self, blocks: &Vec<Block>) -> Result<(), BlockchainError>;
    fn get_height(&self) -> Result<usize, BlockchainError>;
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
    pub fn new(kv_store: K) -> Result<KvStoreChain<K>, BlockchainError> {
        let mut chain = KvStoreChain::<K> { database: kv_store };
        chain.initialize()?;
        Ok(chain)
    }
    fn initialize(&mut self) -> Result<(), BlockchainError> {
        if self.get_height()? == 0 {
            self.apply_block(&genesis::get_genesis_block())?;
        }
        Ok(())
    }

    fn apply_tx(&self, _tx: &Transaction) -> Result<Vec<WriteOp>, BlockchainError> {
        Ok(Vec::new())
    }

    fn apply_block(&mut self, block: &Block) -> Result<(), BlockchainError> {
        let curr_height = self.get_height()?;
        let mut changes = Vec::new();
        for tx in block.body.iter() {
            changes.extend(self.apply_tx(tx)?);
        }
        changes.push(WriteOp::Put(
            StringKey::new("height"),
            (curr_height + 1).into(),
        ));
        self.database.batch(changes)?;
        Ok(())
    }
}

impl<K: KvStore> Blockchain for KvStoreChain<K> {
    fn get_balance(&self, addr: Address) -> Result<Money, BlockchainError> {
        Ok(match self.database.get(addr.get_key())? {
            Some(b) => b.try_into()?,
            None => 0,
        })
    }
    fn extend(&mut self, blocks: &Vec<Block>) -> Result<(), BlockchainError> {
        self.initialize()?;
        for block in blocks.iter() {
            self.apply_block(block)?;
        }
        unimplemented!();
    }
    fn get_height(&self) -> Result<usize, BlockchainError> {
        Ok(match self.database.get(StringKey::new("height"))? {
            Some(b) => b.try_into()?,
            None => 0,
        })
    }
}
