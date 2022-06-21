use thiserror::Error;

use crate::core::ContractId;
use crate::db::{KvStore, KvStoreError, RamKvStore, RamMirrorKvStore, StringKey, WriteOp};
use crate::zk;
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct StateManagerConfig {}

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
    #[error("locator parse error: {0}")]
    LocatorParseError(#[from] zk::ParseZkDataLocatorError),
    #[error("rollback resulted in an invalid root")]
    RollbackToInvalidRoot,
}

pub struct KvStoreStateManager<K: KvStore, H: zk::ZkHasher> {
    config: StateManagerConfig,
    database: K,
    _hasher: std::marker::PhantomData<H>,
}

pub fn compress_state<H: zk::ZkHasher>(
    data_type: zk::ZkStateModel,
    data: zk::ZkDataPairs,
) -> Result<zk::ZkCompressedState, StateManagerError> {
    let id =
        ContractId::from_str("0000000000000000000000000000000000000000000000000000000000000000")
            .unwrap();
    let mut db =
        KvStoreStateManager::<RamKvStore, H>::new(RamKvStore::new(), StateManagerConfig {})?;
    db.new_contract(id, data_type)?;
    db.update_contract(id, &data)?;
    Ok(db.root(id)?)
}

impl<K: KvStore, H: zk::ZkHasher> KvStoreStateManager<K, H> {
    pub fn new(
        database: K,
        config: StateManagerConfig,
    ) -> Result<KvStoreStateManager<K, H>, StateManagerError> {
        let chain = KvStoreStateManager::<K, H> {
            database,
            config: config.clone(),
            _hasher: std::marker::PhantomData,
        };
        Ok(chain)
    }

    pub fn delete_contract(&mut self, id: ContractId) -> Result<(), StateManagerError> {
        let mut rems = Vec::new();
        for (k, _) in self.database.pairs(format!("{}", id).into())? {
            rems.push(WriteOp::Remove(k));
        }
        self.database.update(&rems)?;
        Ok(())
    }

    pub fn new_contract(
        &mut self,
        id: ContractId,
        data_type: zk::ZkStateModel,
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

    fn type_of(&self, id: ContractId) -> Result<zk::ZkStateModel, StateManagerError> {
        Ok(self
            .database
            .get(format!("{}", id).into())?
            .ok_or(StateManagerError::ContractNotFound)?
            .try_into()?)
    }

    pub fn fork_on_ram(&self) -> KvStoreStateManager<RamMirrorKvStore<'_, K>, H> {
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

    pub fn rollback_contract(
        &mut self,
        id: ContractId,
        expected: zk::ZkCompressedState,
    ) -> Result<(), StateManagerError> {
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

        if fork.root(id)? == expected {
            self.database.update(&fork.database.to_ops())?;
        } else {
            return Err(StateManagerError::RollbackToInvalidRoot);
        }
        Ok(())
    }

    pub fn delta_of(
        &self,
        id: ContractId,
        away: usize,
    ) -> Result<zk::ZkDataPairs, StateManagerError> {
        // TODO
        unimplemented!();
    }

    pub fn get_full_state(&self, id: ContractId) -> Result<zk::ZkState, StateManagerError> {
        const MAX_ROLLBACKS: u64 = 5;
        let mut data = zk::ZkDataPairs(Default::default());
        for (k, v) in self.database.pairs(format!("{}_s_", id).into())? {
            let loc = zk::ZkDataLocator::from_str(k.0.split("_").nth(2).unwrap())?;
            data.0.insert(loc, Some(v.try_into()?));
        }
        let mut rollbacks = Vec::<zk::ZkDataPairs>::new();
        let height = self.root(id)?.height;
        for i in 0..MAX_ROLLBACKS {
            if height >= i + 1 {
                rollbacks.push(
                    match self
                        .database
                        .get(format!("{}_rollback_{}", id, height - i - 1).into())?
                    {
                        Some(b) => b.try_into()?,
                        None => {
                            break;
                        }
                    },
                );
            } else {
                break;
            }
        }
        Ok(zk::ZkState { data, rollbacks })
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

        self.database.update(&fork.database.to_ops())?;

        Ok(())
    }

    pub fn update_contract(
        &mut self,
        id: ContractId,
        patch: &zk::ZkDataPairs,
    ) -> Result<(), StateManagerError> {
        const MAX_ROLLBACKS: u64 = 5;
        let mut rollback_patch = zk::ZkDataPairs(HashMap::new());
        let mut fork = self.fork_on_ram();
        let mut root = fork.root(id)?;
        for (k, v) in &patch.0 {
            let prev_val = self.get_data(id, &k)?;
            rollback_patch.0.insert(k.clone(), Some(prev_val)); // Or None if default
            root.state_hash = fork.set_data(id, k.clone(), v.unwrap_or_default())?;
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

        if contract_type.locate(&locator) != zk::ZkStateModel::Scalar {
            return Err(StateManagerError::LocatorError);
        }

        ops.push(if value == zk::ZkScalar::default() {
            WriteOp::Remove(format!("{}_s_{}", id, locator).into())
        } else {
            WriteOp::Put(format!("{}_s_{}", id, locator).into(), value.into())
        });

        while let Some(curr_loc) = locator.0.pop() {
            let curr_type = contract_type.locate(&locator);
            match curr_type.clone() {
                zk::ZkStateModel::List {
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
                                            "{}_{}_aux_{}",
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
                            let aux_key = format!("{}_{}_aux_{}", id, locator, parent_index);
                            ops.push(if value == default_value {
                                WriteOp::Remove(aux_key.into())
                            } else {
                                WriteOp::Put(aux_key.into(), value.into())
                            });
                        }
                    }
                }
                zk::ZkStateModel::Struct { field_types } => {
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
                zk::ZkStateModel::Scalar => {
                    panic!()
                }
            }

            ops.push(if value == curr_type.compress_default::<H>() {
                WriteOp::Remove(format!("{}_{}", id, locator).into())
            } else {
                WriteOp::Put(format!("{}_{}", id, locator).into(), value.into())
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
            match self.database.get(
                format!(
                    "{}_{}{}",
                    cid,
                    if sub_type == zk::ZkStateModel::Scalar {
                        "s_"
                    } else {
                        ""
                    },
                    locator
                )
                .into(),
            )? {
                Some(b) => b.try_into()?,
                None => sub_type.compress_default::<H>(),
            },
        )
    }
}
