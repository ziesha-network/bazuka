use thiserror::Error;

use crate::core::ContractId;
use crate::db::{KvStore, KvStoreError, RamKvStore, StringKey, WriteOp};
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

#[derive(Clone)]
pub struct KvStoreStateManager<H: zk::ZkHasher> {
    config: StateManagerConfig,
    _hasher: std::marker::PhantomData<H>,
}

pub fn compress_state<H: zk::ZkHasher>(
    data_type: zk::ZkStateModel,
    data: zk::ZkDataPairs,
) -> Result<zk::ZkCompressedState, StateManagerError> {
    let id =
        ContractId::from_str("0000000000000000000000000000000000000000000000000000000000000000")
            .unwrap();
    let mut db = KvStoreStateManager::<H>::new(StateManagerConfig {})?;
    let mut ram = RamKvStore::new();
    db.new_contract(
        &mut ram,
        id,
        zk::ZkContract {
            initial_state: zk::ZkCompressedState::empty::<H>(data_type.clone()).into(),
            state_model: data_type.clone(),
            deposit_withdraw_function: zk::ZkVerifierKey::Dummy,
            functions: vec![],
        },
    )?;
    db.update_contract(&mut ram, id, &data.as_delta())?;
    Ok(db.root(&ram, id)?)
}

impl<H: zk::ZkHasher> KvStoreStateManager<H> {
    pub fn new(config: StateManagerConfig) -> Result<KvStoreStateManager<H>, StateManagerError> {
        let chain = KvStoreStateManager::<H> {
            config,
            _hasher: std::marker::PhantomData,
        };
        Ok(chain)
    }

    pub fn delete_contract<K: KvStore>(
        &mut self,
        db: &mut K,
        id: ContractId,
    ) -> Result<(), StateManagerError> {
        let mut rems = Vec::new();
        for (k, _) in db.pairs(format!("{}", id).into())? {
            rems.push(WriteOp::Remove(k));
        }
        db.update(&rems)?;
        Ok(())
    }

    pub fn new_contract<K: KvStore>(
        &mut self,
        db: &mut K,
        id: ContractId,
        contract: zk::ZkContract,
    ) -> Result<(), StateManagerError> {
        db.update(&[WriteOp::Put(
            format!("contract_{}", id).into(),
            contract.clone().into(),
        )])?;
        Ok(())
    }

    fn type_of<K: KvStore>(
        &self,
        db: &K,
        id: ContractId,
    ) -> Result<zk::ZkStateModel, StateManagerError> {
        let cont: zk::ZkContract = db
            .get(format!("contract_{}", id).into())?
            .ok_or(StateManagerError::ContractNotFound)?
            .try_into()?;
        Ok(cont.state_model)
    }

    pub fn root<K: KvStore>(
        &self,
        db: &K,
        id: ContractId,
    ) -> Result<zk::ZkCompressedState, StateManagerError> {
        if let Some(blob) = db.get(format!("{}_compressed", id).into())? {
            Ok(blob.try_into()?)
        } else {
            Ok(zk::ZkCompressedState::empty::<H>(self.type_of(db, id)?))
        }
    }

    pub fn rollback_contract<K: KvStore>(
        &mut self,
        db: &mut K,
        id: ContractId,
    ) -> Result<Option<zk::ZkCompressedState>, StateManagerError> {
        let root = self.root(db, id)?;
        let rollback_key: StringKey = format!("{}_rollback_{}", id, root.height - 1).into();
        let rollback_patch = if let Some(patch) = self.delta_of(db, id, 1)? {
            patch
        } else {
            return Ok(None);
        };
        let mut state_hash = self.root(db, id)?.state_hash;
        for (k, v) in rollback_patch.0 {
            state_hash = self.set_data(db, id, k, v.unwrap_or_default())?;
        }
        let new_state = zk::ZkCompressedState::new(root.height - 1, state_hash, root.state_size);
        db.update(&[
            WriteOp::Remove(rollback_key),
            WriteOp::Put(format!("{}_compressed", id).into(), new_state.into()),
        ])?;

        Ok(Some(new_state))
    }

    pub fn delta_of<K: KvStore>(
        &self,
        db: &K,
        id: ContractId,
        away: u64,
    ) -> Result<Option<zk::ZkDeltaPairs>, StateManagerError> {
        let root = self.root(db, id)?;
        let rollback_key: StringKey = format!("{}_rollback_{}", id, root.height - away).into();
        Ok(match db.get(rollback_key.clone())? {
            Some(b) => Some(b.try_into()?),
            None => None,
        })
    }

    pub fn get_full_state<K: KvStore>(
        &self,
        db: &K,
        id: ContractId,
    ) -> Result<zk::ZkState, StateManagerError> {
        const MAX_ROLLBACKS: u64 = 5;
        let mut data = zk::ZkDataPairs(Default::default());
        for (k, v) in db.pairs(format!("{}_s_", id).into())? {
            let loc = zk::ZkDataLocator::from_str(k.0.split("_").nth(2).unwrap())?;
            data.0.insert(loc, v.try_into()?);
        }
        let mut rollbacks = Vec::<zk::ZkDeltaPairs>::new();
        let height = self.root(db, id)?.height;
        for i in 0..MAX_ROLLBACKS {
            if height >= i + 1 {
                rollbacks.push(
                    match db.get(format!("{}_rollback_{}", id, height - i - 1).into())? {
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

    pub fn reset_contract<K: KvStore>(
        &mut self,
        db: &mut K,
        id: ContractId,
        height: u64,
        state: &zk::ZkState,
    ) -> Result<(zk::ZkCompressedState, Vec<zk::ZkCompressedState>), StateManagerError> {
        let contract_type = self.type_of(db, id)?;
        for (k, _) in db.pairs(format!("{}_", id).into())? {
            db.update(&[WriteOp::Remove(k)])?;
        }

        let mut state_hash = contract_type.compress_default::<H>();
        for (k, v) in state.data.0.iter() {
            state_hash = self.set_data(db, id, k.clone(), *v)?;
        }

        db.update(&[WriteOp::Put(
            format!("{}_compressed", id).into(),
            state_hash.into(),
        )])?;

        let mut rollback_results = Vec::new();

        for (i, rollback) in state.rollbacks.iter().enumerate() {
            let mut state_hash = self.root(db, id)?.state_hash;
            for (k, v) in &rollback.0 {
                state_hash = self.set_data(db, id, k.clone(), v.unwrap_or_default())?;
            }
            rollback_results.push(zk::ZkCompressedState {
                height: height - 1 - i as u64,
                state_hash,
                state_size: 0,
            });
        }

        Ok((
            zk::ZkCompressedState {
                height,
                state_hash,
                state_size: 0,
            },
            rollback_results,
        ))
    }

    pub fn update_contract<K: KvStore>(
        &mut self,
        db: &mut K,
        id: ContractId,
        patch: &zk::ZkDeltaPairs,
    ) -> Result<(), StateManagerError> {
        const MAX_ROLLBACKS: u64 = 5;
        let mut rollback_patch = zk::ZkDeltaPairs(HashMap::new());
        let mut fork = db.mirror();
        let mut root = self.root(&fork, id)?;
        for (k, v) in &patch.0 {
            let prev_val = self.get_data(&fork, id, &k)?;
            rollback_patch.0.insert(k.clone(), Some(prev_val)); // Or None if default
            root.state_hash = self.set_data(&mut fork, id, k.clone(), v.unwrap_or_default())?;
        }
        let mut ops = fork.to_ops();
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
        db.update(&ops)?;
        Ok(())
    }

    fn set_data<K: KvStore>(
        &mut self,
        db: &mut K,
        id: ContractId,
        mut locator: zk::ZkDataLocator,
        mut value: zk::ZkScalar,
    ) -> Result<zk::ZkScalar, StateManagerError> {
        let contract_type = self.type_of(db, id)?;
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
                                    self.get_data(db, id, &full_loc)?
                                } else {
                                    match db.get(
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
                            self.get_data(db, id, &full_loc)?
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

        db.update(&ops)?;
        Ok(value)
    }

    pub fn get_data<K: KvStore>(
        &self,
        db: &K,
        cid: ContractId,
        locator: &zk::ZkDataLocator,
    ) -> Result<zk::ZkScalar, StateManagerError> {
        let sub_type = self.type_of(db, cid)?.locate(locator);
        Ok(
            match db.get(
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
