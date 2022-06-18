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
    #[error("not locating a scalar")]
    LocatorError,
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
        self.database.update(&[
            WriteOp::Put(format!("{}", id).into(), data_type.clone().into()),
            WriteOp::Put(
                format!("{}_compressed", id).into(),
                zk::ZkCompressedState::empty(data_type).into(),
            ),
        ])?;
        Ok(())
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

    pub fn root(&self, id: ContractId) -> Result<zk::ZkCompressedState, StateManagerError> {
        Ok(self
            .database
            .get(format!("{}_compressed", id).into())?
            .ok_or(StateManagerError::ContractNotFound)?
            .try_into()?)
    }

    fn set_data(
        &mut self,
        id: ContractId,
        mut locator: Vec<zk::ZkDataLocator>,
        mut value: zk::ZkScalar,
    ) -> Result<(), StateManagerError> {
        let contract_type = self.type_of(id)?;
        let mut ops = Vec::new();
        if contract_type.locate(&locator) != zk::ZkDataType::Scalar {
            return Err(StateManagerError::LocatorError);
        }

        ops.push(WriteOp::Put(
            format!("{}_{:?}", id, locator).into(),
            value.into(),
        ));

        while let Some(curr_loc) = locator.pop() {
            let curr_type = contract_type.locate(&locator);
            match curr_type {
                zk::ZkDataType::List {
                    item_type,
                    log4_size,
                } => {
                    if let zk::ZkDataLocator::Leaf { leaf_index } = curr_loc {
                        let mut curr_ind = leaf_index;
                        let mut default_value = item_type.compress_default();
                        for layer in log4_size..2 {
                            if layer == log4_size {
                                let mut dats = Vec::new();
                                let start = curr_ind - (curr_ind % 4);
                                for leaf_index in start..start + 4 {
                                    let leaf_loc = zk::ZkDataLocator::Leaf {
                                        leaf_index: leaf_index as u32,
                                    };
                                    dats.push(if leaf_index == curr_ind {
                                        value
                                    } else {
                                        let mut full_loc = locator.clone();
                                        full_loc.push(leaf_loc);
                                        self.get_data(id, &full_loc)?
                                    });
                                }
                                value = zk::ZkScalar(zeekit::mimc::mimc(
                                    &dats.into_iter().map(|d| d.0).collect::<Vec<_>>(),
                                ));
                            } else {
                                let mut dats = Vec::new();
                                let mut default_dats = Vec::new();
                                let aux_offset = (1 << (2 * (layer - 1)) - 1) / 3;

                                let start = curr_ind - (curr_ind % 4);
                                for leaf_index in start..start + 4 {
                                    default_dats.push(default_value);
                                    dats.push(if leaf_index == curr_ind {
                                        value
                                    } else {
                                        match self.database.get(
                                            format!(
                                                "{}_{:?}_aux_{}",
                                                id,
                                                locator,
                                                aux_offset + leaf_index
                                            )
                                            .into(),
                                        )? {
                                            Some(b) => b.try_into()?,
                                            None => default_value,
                                        }
                                    });
                                }
                                default_value = zk::ZkScalar(zeekit::mimc::mimc(
                                    &default_dats.into_iter().map(|d| d.0).collect::<Vec<_>>(),
                                ));
                                value = zk::ZkScalar(zeekit::mimc::mimc(
                                    &dats.into_iter().map(|d| d.0).collect::<Vec<_>>(),
                                ));
                            }

                            curr_ind = curr_ind / 4;
                            let parent_aux_offset = (1 << (2 * (layer - 2)) - 1) / 3;
                            let parent_index = parent_aux_offset + curr_ind;

                            ops.push(WriteOp::Put(
                                format!("{}_{:?}_aux_{}", id, locator, parent_index).into(),
                                value.into(),
                            ));
                        }
                    } else {
                        panic!();
                    }
                }
                zk::ZkDataType::Struct { field_types } => {
                    let mut dats = Vec::new();
                    for field_index in 0..field_types.len() {
                        let field_loc = zk::ZkDataLocator::Field {
                            field_index: field_index as u32,
                        };
                        dats.push(if field_loc == curr_loc {
                            value
                        } else {
                            let mut full_loc = locator.clone();
                            full_loc.push(field_loc);
                            self.get_data(id, &full_loc)?
                        });
                    }
                    value = zk::ZkScalar(zeekit::mimc::mimc(
                        &dats.into_iter().map(|d| d.0).collect::<Vec<_>>(),
                    ));
                }
                zk::ZkDataType::Scalar => {
                    panic!()
                }
            }

            ops.push(WriteOp::Put(
                format!("{}_{:?}", id, locator).into(),
                value.into(),
            ));
        }
        self.database.update(&ops)?;
        Ok(())
    }

    pub fn get_data(
        &self,
        cid: ContractId,
        locator: &[zk::ZkDataLocator],
    ) -> Result<zk::ZkScalar, StateManagerError> {
        let sub_type = self.type_of(cid)?.locate(&locator);
        Ok(
            match self.database.get(format!("{}_{:?}", cid, locator).into())? {
                Some(b) => b.try_into()?,
                None => sub_type.compress_default(),
            },
        )
    }
}
