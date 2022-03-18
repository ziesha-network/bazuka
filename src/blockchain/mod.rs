use crate::config::{genesis, TOTAL_SUPPLY};
use crate::core::{Account, Address, Block, Header, Transaction, TransactionData};
use crate::db::{KvStore, KvStoreError, RamMirrorKvStore, StringKey, WriteOp};
use crate::wallet::Wallet;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BlockchainError {
    #[error("kvstore error happened")]
    KvStoreError(#[from] KvStoreError),
    #[error("transaction signature is invalid")]
    SignatureError,
    #[error("balance insufficient")]
    BalanceInsufficient,
    #[error("inconsistency error")]
    Inconsistency,
}

pub trait Blockchain {
    fn get_account(&self, addr: Address) -> Result<Account, BlockchainError>;
    fn extend(&mut self, blocks: &Vec<Block>) -> Result<(), BlockchainError>;
    fn generate_block(
        &mut self,
        mempool: &Vec<Transaction>,
        wallet: &Wallet,
    ) -> Result<Option<Block>, BlockchainError>;
    fn get_height(&self) -> Result<usize, BlockchainError>;
    fn get_headers(
        &self,
        since: usize,
        until: Option<usize>,
    ) -> Result<Vec<Header>, BlockchainError>;
    fn get_blocks(&self, since: usize, until: Option<usize>)
        -> Result<Vec<Block>, BlockchainError>;
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
                let mut acc_src = self.get_account(tx.src.clone())?;

                if acc_src.balance < amount + tx.fee {
                    return Err(BlockchainError::BalanceInsufficient);
                }

                acc_src.balance -= if *dst != tx.src { *amount } else { 0 } + tx.fee;
                acc_src.nonce += 1;

                ops.push(WriteOp::Put(
                    format!("account_{}", tx.src).into(),
                    acc_src.into(),
                ));

                if *dst != tx.src {
                    let mut acc_dst = self.get_account(dst.clone())?;
                    acc_dst.balance += amount;

                    ops.push(WriteOp::Put(
                        format!("account_{}", dst).into(),
                        acc_dst.into(),
                    ));
                }
            }
            _ => {
                unimplemented!();
            }
        }
        Ok(ops)
    }

    pub fn rollback_block(&mut self) -> Result<(), BlockchainError> {
        let height = self.get_height()?;
        let rollback_key: StringKey = format!("rollback_{:010}", height - 1).into();
        let mut rollback: Vec<WriteOp> = match self.database.get(rollback_key.clone())? {
            Some(b) => b.try_into()?,
            None => {
                return Err(BlockchainError::Inconsistency);
            }
        };
        rollback.push(WriteOp::Remove(rollback_key));
        self.database.batch(rollback)?;
        Ok(())
    }

    fn apply_block(&mut self, block: &Block) -> Result<(), BlockchainError> {
        let curr_height = self.get_height()?;
        let mut changes = Vec::new();
        for tx in block.body.iter() {
            changes.extend(self.apply_tx(tx)?);
        }
        changes.push(WriteOp::Put(
            format!("block_{:010}", block.header.number).into(),
            block.into(),
        ));
        changes.push(WriteOp::Put("height".into(), (curr_height + 1).into()));

        changes.push(WriteOp::Put(
            format!("rollback_{:010}", block.header.number).into(),
            self.database.gen_rollback(&changes)?.into(),
        ));

        self.database.batch(changes)?;
        Ok(())
    }
}

impl<K: KvStore> Blockchain for KvStoreChain<K> {
    fn get_account(&self, addr: Address) -> Result<Account, BlockchainError> {
        let k = format!("account_{}", addr).into();
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
        Ok(match self.database.get("height".into())? {
            Some(b) => b.try_into()?,
            None => 0,
        })
    }
    fn get_headers(
        &self,
        since: usize,
        until: Option<usize>,
    ) -> Result<Vec<Header>, BlockchainError> {
        Ok(self
            .get_blocks(since, until)?
            .into_iter()
            .map(|b| b.header)
            .collect())
    }
    fn get_blocks(
        &self,
        since: usize,
        until: Option<usize>,
    ) -> Result<Vec<Block>, BlockchainError> {
        let mut blks: Vec<Block> = Vec::new();
        let height = self.get_height()?;
        for i in since..until.unwrap_or(height) {
            if i >= height {
                break;
            }
            blks.push(
                self.database
                    .get(format!("block_{:010}", i).into())?
                    .ok_or(BlockchainError::Inconsistency)?
                    .try_into()?,
            );
        }
        Ok(blks)
    }
    fn generate_block(
        &mut self,
        _mempool: &Vec<Transaction>,
        _wallet: &Wallet,
    ) -> Result<Option<Block>, BlockchainError> {
        Ok(None)
    }
}
