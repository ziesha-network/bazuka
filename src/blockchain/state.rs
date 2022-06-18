use thiserror::Error;

use super::BlockchainConfig;

use crate::core::{
    hash::Hash, Account, Address, Block, ContractAccount, ContractId, ContractUpdate, Hasher,
    Header, Money, ProofOfWork, Signature, Transaction, TransactionAndDelta, TransactionData,
};
use crate::db::{KvStore, KvStoreError, RamMirrorKvStore, StringKey, WriteOp};
use crate::zk;

#[derive(Error, Debug)]
pub enum StateManagerError {
    #[error("kvstore error happened: {0}")]
    KvStoreError(#[from] KvStoreError),
    #[error("contract not found")]
    ContractNotFound,
    #[error("data not found")]
    NotFound,
}

pub struct KvStoreStateManager<K: KvStore> {
    config: BlockchainConfig,
    database: K,
}

pub struct ContractStats {
    height: u32,
    size: u32,
}

impl<K: KvStore> KvStoreStateManager<K> {
    pub fn new(
        database: K,
        config: BlockchainConfig,
    ) -> Result<KvStoreStateManager<K>, StateManagerError> {
        let mut chain = KvStoreStateManager::<K> {
            database,
            config: config.clone(),
        };
        Ok(chain)
    }

    pub fn new_contract(
        &mut self,
        id: ContractId,
        data_type: zk::ZkDataType,
    ) -> Result<(), StateManagerError> {
        self.database
            .update(&[WriteOp::Put(format!("{}", id).into(), data_type.into())])?;
        Err(StateManagerError::NotFound)
    }

    fn type_of(&self, id: ContractId) -> Result<zk::ZkDataType, StateManagerError> {
        Ok(self
            .database
            .get(format!("{}", id).into())?
            .ok_or(StateManagerError::ContractNotFound)?
            .try_into()?)
    }

    fn fork_on_ram(&self) -> KvStoreStateManager<RamMirrorKvStore<'_, K>> {
        KvStoreStateManager {
            database: RamMirrorKvStore::new(&self.database),
            config: self.config.clone(),
        }
    }

    pub fn compress(&self, cid: ContractId) -> Result<zk::ZkCompressedState, StateManagerError> {
        Err(StateManagerError::NotFound)
    }

    fn set_data(
        &self,
        cid: ContractId,
        locator: Vec<zk::ZkDataLocator>,
        value: zk::ZkScalar,
    ) -> Result<(), StateManagerError> {
        Err(StateManagerError::NotFound)
    }

    pub fn get_data(
        &self,
        cid: ContractId,
        locator: Vec<zk::ZkDataLocator>,
    ) -> Result<zk::ZkScalar, StateManagerError> {
        let addr = locator
            .into_iter()
            .map(|l| match l {
                zk::ZkDataLocator::Field { field_index } => field_index,
                zk::ZkDataLocator::Leaf { leaf_index } => leaf_index,
            })
            .collect::<Vec<u32>>();
        Err(StateManagerError::NotFound)
    }
}
