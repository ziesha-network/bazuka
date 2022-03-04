use crate::config::{genesis, TOTAL_SUPPLY};
use crate::core::{Account, Address, Block, Transaction, TransactionData};
use crate::db::{KvStore, KvStoreError, RamMirrorKvStore, StringKey, WriteOp};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BlockchainError {
    #[error("kvstore error happened")]
    KvStoreError(#[from] KvStoreError),
    #[error("transaction signature is invalid")]
    SignatureError,
    #[error("balance insufficient")]
    BalanceInsufficient,
}

pub trait Blockchain {
    fn get_account(&self, addr: Address) -> Result<Account, BlockchainError>;
    fn extend(&mut self, blocks: &Vec<Block>) -> Result<(), BlockchainError>;
    fn get_height(&self) -> Result<usize, BlockchainError>;
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

    fn fork<'a>(&'a self) -> Result<KvStoreChain<RamMirrorKvStore<'a, K>>, BlockchainError> {
        KvStoreChain::new(RamMirrorKvStore::new(&self.database))
    }

    fn initialize(&mut self) -> Result<(), BlockchainError> {
        if self.get_height()? == 0 {
            self.apply_block(&genesis::get_genesis_block())?;
        }
        Ok(())
    }

    fn apply_tx(&self, tx: &Transaction) -> Result<Vec<WriteOp>, BlockchainError> {
        let mut ops = Vec::new();
        if !tx.verify_signature() {
            return Err(BlockchainError::SignatureError);
        }
        match &tx.data {
            TransactionData::RegularSend { dst, amount } => {
                // WARN: Fails when dst == src

                let mut acc_src = self.get_account(tx.src.clone())?;

                if acc_src.balance < amount + tx.fee {
                    return Err(BlockchainError::BalanceInsufficient);
                }

                acc_src.balance -= amount + tx.fee;

                ops.push(WriteOp::Put(
                    StringKey::new(&format!("account_{}", tx.src)),
                    acc_src.into(),
                ));

                let mut acc_dst = self.get_account(dst.clone())?;
                acc_dst.balance += amount;
                acc_dst.nonce += 1;

                ops.push(WriteOp::Put(
                    StringKey::new(&format!("account_{}", dst)),
                    acc_dst.into(),
                ));
            }
        }
        Ok(ops)
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
    fn get_account(&self, addr: Address) -> Result<Account, BlockchainError> {
        let k = StringKey::new(&format!("account_{}", addr));
        Ok(match self.database.get(k)? {
            Some(b) => b.try_into()?,
            None => Account {
                balance: if addr == Address::Treasury {
                    TOTAL_SUPPLY
                } else {
                    0
                },
                nonce: 0,
            },
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
