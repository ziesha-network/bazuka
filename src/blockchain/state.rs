use thiserror::Error;

use super::BlockchainConfig;

use crate::core::ContractId;
use crate::db::{KvStore, KvStoreError, RamMirrorKvStore, StringKey, WriteOp};
use crate::zk;
use std::collections::HashMap;

#[derive(Error, Debug)]
pub enum StateManagerError {
    #[error("kvstore error happened: {0}")]
    KvStoreError(#[from] KvStoreError),
    #[error("contract not found")]
    ContractNotFound,
    #[error("rollback not found")]
    RollbackNotFound,
    #[error("data does not correspond to target")]
    TargetMismatch,
    #[error("not locating a scalar")]
    LocatorError,
}

pub struct KvStoreStateManager<K: KvStore, H: zk::ZkHasher> {
    config: BlockchainConfig,
    database: K,
    _hasher: std::marker::PhantomData<H>,
}

impl<K: KvStore, H: zk::ZkHasher> KvStoreStateManager<K, H> {
    pub fn new(
        database: K,
        config: BlockchainConfig,
    ) -> Result<KvStoreStateManager<K, H>, StateManagerError> {
        let chain = KvStoreStateManager::<K, H> {
            database,
            config: config.clone(),
            _hasher: std::marker::PhantomData,
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
                zk::ZkCompressedState::empty::<H>(data_type).into(),
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

    fn fork_on_ram(&self) -> KvStoreStateManager<RamMirrorKvStore<'_, K>, H> {
        KvStoreStateManager {
            database: RamMirrorKvStore::new(&self.database),
            config: self.config.clone(),
            _hasher: self._hasher,
        }
    }

    pub fn root(&self, id: ContractId) -> Result<zk::ZkCompressedState, StateManagerError> {
        Ok(self
            .database
            .get(format!("{}_compressed", id).into())?
            .ok_or(StateManagerError::ContractNotFound)?
            .try_into()?)
    }

    pub fn rollback_contract(&mut self, id: ContractId) -> Result<(), StateManagerError> {
        let root = self.root(id)?;
        let rollback_key: StringKey = format!("{}_rollback_{}", id, root.height - 1).into();
        let mut rollback_patch: zk::ZkDataPairs = match self.database.get(rollback_key.clone())? {
            Some(b) => b.try_into()?,
            None => {
                return Err(StateManagerError::RollbackNotFound);
            }
        };
        let mut fork = self.fork_on_ram();
        let mut state_hash = self.root(id)?.state_hash;
        for (k, v) in rollback_patch.0 {
            state_hash = fork.set_data(id, k, v.unwrap_or_default())?;
        }
        fork.database.update(&[
            WriteOp::Remove(rollback_key),
            WriteOp::Put(
                format!("{}_compressed", id).into(),
                zk::ZkCompressedState::new(root.height - 1, state_hash, root.state_size).into(),
            ),
        ])?;

        self.database.update(&fork.database.to_ops())?;
        Ok(())
    }

    pub fn reset_contract(
        &mut self,
        id: ContractId,
        data: zk::ZkDataPairs,
        data_target: zk::ZkCompressedState,
        rollbacks: Vec<zk::ZkDataPairs>,
        rollback_targets: Vec<zk::ZkCompressedState>,
    ) -> Result<(), StateManagerError> {
        let contract_type = self.type_of(id)?;
        let mut fork = self.fork_on_ram();
        for (k, _) in fork.database.pairs(format!("{}_", id).into())? {
            fork.database.update(&[WriteOp::Remove(k)])?;
        }

        let mut state_hash = contract_type.compress_default::<H>();
        for (k, v) in data.0 {
            state_hash = fork.set_data(id, k, v.unwrap_or_default())?;
        }

        if state_hash != data_target.state_hash || data_target.state_size != 0 {
            return Err(StateManagerError::TargetMismatch);
        }

        fork.database.update(&[WriteOp::Put(
            format!("{}_compressed", id).into(),
            data_target.into(),
        )])?;

        let mut rollback_updates = Vec::new();

        let mut rollback_fork = fork.fork_on_ram();
        for (i, (rollback, rollback_target)) in rollbacks
            .iter()
            .zip(rollback_targets.into_iter())
            .enumerate()
        {
            let mut state_hash = rollback_fork.root(id)?.state_hash;
            for (k, v) in &rollback.0 {
                state_hash = rollback_fork.set_data(id, k.clone(), v.unwrap_or_default())?;
            }
            if state_hash != rollback_target.state_hash
                || rollback_target.state_size != 0
                || rollback_target.height != data_target.height - 1 - i as u64
            {
                return Err(StateManagerError::TargetMismatch);
            }
            rollback_updates.push(WriteOp::Put(
                format!("{}_rollback_{}", id, rollback_target.height).into(),
                rollback.into(),
            ));
        }

        fork.database.update(&rollback_updates)?;

        self.database.update(&fork.database.to_ops());

        Ok(())
    }

    pub fn update_contract(
        &mut self,
        id: ContractId,
        patch: zk::ZkDataPairs,
    ) -> Result<(), StateManagerError> {
        const MAX_ROLLBACKS: u64 = 5;
        let mut rollback_patch = zk::ZkDataPairs(HashMap::new());
        let mut fork = self.fork_on_ram();
        let mut root = fork.root(id)?;
        for (k, v) in patch.0 {
            let prev_val = self.get_data(id, &k)?;
            rollback_patch.0.insert(k.clone(), Some(prev_val)); // Or None if default
            root.state_hash = fork.set_data(id, k, v.unwrap_or_default())?;
        }
        let mut ops = fork.database.to_ops();
        ops.push(WriteOp::Put(
            format!("{}_compressed", id).into(),
            zk::ZkCompressedState::new(root.height + 1, root.state_hash, root.state_size).into(),
        ));
        ops.push(WriteOp::Put(
            format!("{}_rollback_{}", id, root.height).into(),
            (&rollback_patch).into(),
        ));
        if root.height >= MAX_ROLLBACKS {
            ops.push(WriteOp::Remove(
                format!("{}_rollback_{}", id, root.height - MAX_ROLLBACKS).into(),
            ));
        }
        self.database.update(&ops)?;
        Ok(())
    }

    fn set_data(
        &mut self,
        id: ContractId,
        mut locator: zk::ZkDataLocator,
        mut value: zk::ZkScalar,
    ) -> Result<zk::ZkScalar, StateManagerError> {
        let contract_type = self.type_of(id)?;
        let mut ops = Vec::new();

        if contract_type.locate(&locator) != zk::ZkDataType::Scalar {
            return Err(StateManagerError::LocatorError);
        }

        ops.push(if value == zk::ZkScalar::default() {
            WriteOp::Remove(format!("{}_{:?}", id, locator).into())
        } else {
            WriteOp::Put(format!("{}_{:?}", id, locator).into(), value.into())
        });

        while let Some(curr_loc) = locator.0.pop() {
            let curr_type = contract_type.locate(&locator);
            match curr_type.clone() {
                zk::ZkDataType::List {
                    item_type,
                    log4_size,
                } => {
                    let leaf_index = curr_loc;
                    let mut curr_ind = leaf_index;
                    let mut default_value = item_type.compress_default::<H>();
                    for layer in (0..log4_size).rev() {
                        let mut dats = Vec::new();
                        let aux_offset = ((1 << (2 * (layer + 1))) - 1) / 3;
                        let start = curr_ind - (curr_ind % 4);
                        for leaf_index in start..start + 4 {
                            dats.push(if leaf_index == curr_ind {
                                value
                            } else {
                                if layer == log4_size - 1 {
                                    let mut full_loc = locator.clone();
                                    full_loc.0.push(leaf_index as u32);
                                    self.get_data(id, &full_loc)?
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
                                }
                            });
                        }

                        value = H::hash(&dats);
                        default_value = H::hash(&[default_value; 4]);

                        curr_ind = curr_ind / 4;

                        if layer > 0 {
                            let parent_aux_offset = ((1 << (2 * layer)) - 1) / 3;
                            let parent_index = parent_aux_offset + curr_ind;
                            let aux_key = format!("{}_{:?}_aux_{}", id, locator, parent_index);
                            ops.push(if value == default_value {
                                WriteOp::Remove(aux_key.into())
                            } else {
                                WriteOp::Put(aux_key.into(), value.into())
                            });
                        }
                    }
                }
                zk::ZkDataType::Struct { field_types } => {
                    let mut dats = Vec::new();
                    for field_index in 0..field_types.len() {
                        dats.push(if field_index as u32 == curr_loc {
                            value
                        } else {
                            let mut full_loc = locator.clone();
                            full_loc.0.push(field_index as u32);
                            self.get_data(id, &full_loc)?
                        });
                    }
                    value = H::hash(&dats);
                }
                zk::ZkDataType::Scalar => {
                    panic!()
                }
            }

            ops.push(if value == curr_type.compress_default::<H>() {
                WriteOp::Remove(format!("{}_{:?}", id, locator).into())
            } else {
                WriteOp::Put(format!("{}_{:?}", id, locator).into(), value.into())
            });
        }

        self.database.update(&ops)?;
        Ok(value)
    }

    pub fn get_data(
        &self,
        cid: ContractId,
        locator: &zk::ZkDataLocator,
    ) -> Result<zk::ZkScalar, StateManagerError> {
        let sub_type = self.type_of(cid)?.locate(locator);
        Ok(
            match self.database.get(format!("{}_{:?}", cid, locator).into())? {
                Some(b) => b.try_into()?,
                None => sub_type.compress_default::<H>(),
            },
        )
    }
}
