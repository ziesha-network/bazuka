use thiserror::Error;

use super::*;
use crate::core::ContractId;
use crate::crypto::jubjub;
use crate::db::{keys, KvStore, KvStoreError, RamKvStore, WriteOp};
use ff::Field;
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Error, Debug)]
pub enum StateManagerError {
    #[error("kvstore error happened: {0}")]
    KvStoreError(#[from] KvStoreError),
    #[error("locator error: {0}")]
    LocatorError(#[from] ZkLocatorError),
    #[error("contract not found")]
    ContractNotFound,
    #[error("not locating a scalar")]
    NonScalarLocatorError,
    #[error("locator parse error: {0}")]
    LocatorParseError(#[from] ParseZkDataLocatorError),
    #[error("not locating a tree")]
    NonTreeLocatorError,
    #[error("zk error: {0}")]
    ZkError(#[from] ZkError),
}

#[derive(Clone)]
pub struct KvStoreStateManager<H: ZkHasher> {
    _hasher: std::marker::PhantomData<H>,
}

pub struct ZkStateBuilder<H: ZkHasher> {
    contract_id: ContractId,
    _hasher: std::marker::PhantomData<H>,
    db: RamKvStore,
}

impl<H: ZkHasher> ZkStateBuilder<H> {
    pub fn new(state_model: ZkStateModel) -> Self {
        let contract_id = ContractId::from_str(
            "0000000000000000000000000000000000000000000000000000000000000000",
        )
        .unwrap();
        let mut db = RamKvStore::new();
        db.update(&[WriteOp::Put(
            keys::contract(&contract_id),
            ZkContract {
                initial_state: ZkCompressedState::empty::<H>(state_model.clone()),
                state_model,
                payment_functions: vec![],
                functions: vec![],
            }
            .into(),
        )])
        .unwrap();
        Self {
            contract_id,
            db,
            _hasher: std::marker::PhantomData,
        }
    }
    pub fn batch_set(&mut self, delta: &ZkDeltaPairs) -> Result<(), StateManagerError> {
        KvStoreStateManager::<H>::update_contract(&mut self.db, self.contract_id, delta)?;
        Ok(())
    }
    pub fn get(&mut self, loc: ZkDataLocator) -> Result<ZkScalar, StateManagerError> {
        KvStoreStateManager::<H>::get_data(&self.db, self.contract_id, &loc)
    }
    pub fn compress(self) -> Result<ZkCompressedState, StateManagerError> {
        KvStoreStateManager::<H>::root(&self.db, self.contract_id)
    }

    pub fn prove(
        &self,
        tree_loc: ZkDataLocator,
        ind: u32,
    ) -> Result<Vec<[ZkScalar; 3]>, StateManagerError> {
        KvStoreStateManager::<H>::prove(&self.db, self.contract_id, tree_loc, ind)
    }
}

impl<H: ZkHasher> KvStoreStateManager<H> {
    pub fn get_mpn_account<K: KvStore>(
        db: &K,
        mpn_contract_id: ContractId,
        index: u32,
    ) -> Result<MpnAccount, StateManagerError> {
        let cells = (0..4)
            .map(|i| Self::get_data(db, mpn_contract_id, &ZkDataLocator(vec![index, i as u32])))
            .collect::<Result<Vec<ZkScalar>, StateManagerError>>()?;
        Ok(MpnAccount {
            nonce: cells[0].try_into()?,
            address: jubjub::PointAffine(cells[1], cells[2]),
            balance: cells[3].try_into()?,
        })
    }

    pub fn set_mpn_account<K: KvStore>(
        db: &mut K,
        mpn_contract_id: ContractId,
        index: u32,
        acc: MpnAccount,
        size_diff: &mut u32,
    ) -> Result<(), StateManagerError> {
        let vals = [
            acc.nonce.into(),
            acc.address.0,
            acc.address.1,
            acc.balance.into(),
        ];
        vals.into_iter()
            .enumerate()
            .map(|(i, val)| {
                Self::set_data(
                    db,
                    mpn_contract_id,
                    ZkDataLocator(vec![index, i as u32]),
                    val,
                    size_diff,
                )
            })
            .collect::<Result<Vec<ZkScalar>, StateManagerError>>()?;
        Ok(())
    }

    pub fn delete_contract<K: KvStore>(
        db: &mut K,
        id: ContractId,
    ) -> Result<(), StateManagerError> {
        let mut rems = Vec::new();
        for (k, _) in db.pairs(keys::local_prefix(&id).into())? {
            rems.push(WriteOp::Remove(k));
        }
        db.update(&rems)?;
        Ok(())
    }

    pub fn height_of<K: KvStore>(db: &K, id: ContractId) -> Result<u64, StateManagerError> {
        if let Some(blob) = db.get(keys::local_height(&id))? {
            Ok(blob.try_into()?)
        } else {
            Ok(0)
        }
    }

    pub fn prove<K: KvStore>(
        db: &K,
        id: ContractId,
        tree_loc: ZkDataLocator,
        mut curr_ind: u32,
    ) -> Result<Vec<[ZkScalar; 3]>, StateManagerError> {
        let loc_type = Self::type_of(db, id)?.locate(&tree_loc)?;
        if let ZkStateModel::List {
            log4_size,
            item_type,
        } = loc_type
        {
            let mut default_value = item_type.compress_default::<H>();
            let mut proof = Vec::new();

            for layer in (0..log4_size).rev() {
                let mut proof_part = [ZkScalar::default(); 3];
                let aux_offset = ((1 << (2 * (layer + 1))) - 1) / 3;
                let start = curr_ind - (curr_ind % 4);
                let mut i = 0;
                for leaf_index in start..start + 4 {
                    if leaf_index != curr_ind {
                        proof_part[i] = if layer == log4_size - 1 {
                            Self::get_data(db, id, &tree_loc.index(leaf_index as u32))?
                        } else {
                            match db.get(keys::local_tree_aux(
                                &id,
                                &tree_loc,
                                aux_offset + leaf_index,
                            ))? {
                                Some(b) => b.try_into()?,
                                None => default_value,
                            }
                        };
                        i += 1;
                    };
                }
                curr_ind /= 4;
                default_value = H::hash(&[default_value; 4]);
                proof.push(proof_part);
            }

            Ok(proof)
        } else {
            Err(StateManagerError::NonTreeLocatorError)
        }
    }

    pub fn type_of<K: KvStore>(db: &K, id: ContractId) -> Result<ZkStateModel, StateManagerError> {
        let cont: ZkContract = db
            .get(keys::contract(&id))?
            .ok_or(StateManagerError::ContractNotFound)?
            .try_into()?;
        Ok(cont.state_model)
    }

    pub fn root<K: KvStore>(
        db: &K,
        id: ContractId,
    ) -> Result<ZkCompressedState, StateManagerError> {
        if let Some(blob) = db.get(keys::local_root(&id))? {
            Ok(blob.try_into()?)
        } else {
            Ok(ZkCompressedState::empty::<H>(Self::type_of(db, id)?))
        }
    }

    pub fn rollback_contract<K: KvStore>(
        db: &mut K,
        id: ContractId,
    ) -> Result<Option<ZkCompressedState>, StateManagerError> {
        let mut root = Self::root(db, id)?;
        let height = Self::height_of(db, id)?;
        let rollback_key = keys::local_rollback_to_height(&id, height);
        let rollback_patch = if let Some(patch) = Self::rollback_of(db, id, 1)? {
            patch
        } else {
            return Ok(None);
        };
        for (k, v) in rollback_patch.0 {
            root.state_hash =
                Self::set_data(db, id, k, v.unwrap_or_default(), &mut root.state_size)?;
        }
        db.update(&[
            WriteOp::Remove(rollback_key),
            WriteOp::Put(keys::local_root(&id), root.into()),
            WriteOp::Put(keys::local_height(&id), (height - 1).into()),
        ])?;

        Ok(Some(root))
    }

    pub fn delta_of<K: KvStore>(
        db: &K,
        id: ContractId,
        away: u64,
    ) -> Result<Option<ZkDeltaPairs>, StateManagerError> {
        let mut data = ZkDeltaPairs(Default::default());
        for i in 0..away {
            if let Some(rollback) = Self::rollback_of(db, id, i + 1)? {
                for (k, _) in rollback.0 {
                    data.0.insert(k.clone(), Some(Self::get_data(db, id, &k)?));
                }
            } else {
                return Ok(None);
            }
        }
        Ok(Some(data))
    }

    pub fn rollback_of<K: KvStore>(
        db: &K,
        id: ContractId,
        away: u64,
    ) -> Result<Option<ZkDeltaPairs>, StateManagerError> {
        let height = Self::height_of(db, id)?;
        Ok(
            match db.get(keys::local_rollback_to_height(&id, height - away))? {
                Some(b) => Some(b.try_into()?),
                None => None,
            },
        )
    }

    pub fn get_full_state<K: KvStore>(
        db: &K,
        id: ContractId,
    ) -> Result<ZkState, StateManagerError> {
        const MAX_ROLLBACKS: u64 = 5;
        let mut data = ZkDataPairs(Default::default());
        for (k, v) in db.pairs(keys::local_scalar_value_prefix(&id).into())? {
            let loc = ZkDataLocator::from_str(k.0.split('_').nth(2).unwrap())?;
            data.0.insert(loc, v.try_into()?);
        }
        let mut rollbacks = Vec::<ZkDeltaPairs>::new();
        let height = Self::height_of(db, id)?;
        for i in 0..MAX_ROLLBACKS {
            if height > i {
                rollbacks.push(
                    match db.get(keys::local_rollback_to_height(&id, height - i - 1))? {
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
        Ok(ZkState { data, rollbacks })
    }

    pub fn reset_contract<K: KvStore>(
        db: &mut K,
        id: ContractId,
        height: u64,
        state: &ZkState,
    ) -> Result<(ZkCompressedState, Vec<ZkCompressedState>), StateManagerError> {
        let contract_type = Self::type_of(db, id)?;
        for (k, _) in db.pairs(keys::local_prefix(&id).into())? {
            db.update(&[WriteOp::Remove(k)])?;
        }

        let mut state_hash = contract_type.compress_default::<H>();
        let mut state_size = 0;
        for (k, v) in state.data.0.iter() {
            state_hash = Self::set_data(db, id, k.clone(), *v, &mut state_size)?;
        }

        db.update(&[
            WriteOp::Put(
                keys::local_root(&id),
                ZkCompressedState {
                    state_hash,
                    state_size,
                }
                .into(),
            ),
            WriteOp::Put(keys::local_height(&id), height.into()),
        ])?;

        let mut rollback_results = Vec::new();
        let mut root = Self::root(db, id)?;

        for (i, rollback) in state.rollbacks.iter().enumerate() {
            for (k, v) in &rollback.0 {
                root.state_hash = Self::set_data(
                    db,
                    id,
                    k.clone(),
                    v.unwrap_or_default(),
                    &mut root.state_size,
                )?;
            }
            db.update(&[WriteOp::Put(
                keys::local_rollback_to_height(&id, height - 1 - i as u64),
                rollback.into(),
            )])?;
            rollback_results.push(root);
        }

        Ok((root, rollback_results))
    }

    pub fn update_contract<K: KvStore>(
        db: &mut K,
        id: ContractId,
        patch: &ZkDeltaPairs,
    ) -> Result<(), StateManagerError> {
        const MAX_ROLLBACKS: u64 = 5;
        let mut rollback_patch = ZkDeltaPairs(HashMap::new());
        let mut fork = db.mirror();
        let mut root = Self::root(&fork, id)?;
        let height = Self::height_of(&fork, id)?;
        for (k, v) in &patch.0 {
            let prev_val = Self::get_data(&fork, id, k)?;
            rollback_patch.0.insert(k.clone(), Some(prev_val)); // Or None if default
            root.state_hash = Self::set_data(
                &mut fork,
                id,
                k.clone(),
                v.unwrap_or_default(),
                &mut root.state_size,
            )?;
        }
        let mut ops = fork.to_ops();
        ops.push(WriteOp::Put(keys::local_root(&id), root.into()));
        ops.push(WriteOp::Put(
            keys::local_rollback_to_height(&id, height),
            (&rollback_patch).into(),
        ));
        ops.push(WriteOp::Put(keys::local_height(&id), (height + 1).into()));
        if height >= MAX_ROLLBACKS {
            ops.push(WriteOp::Remove(keys::local_rollback_to_height(
                &id,
                height - MAX_ROLLBACKS,
            )));
        }
        db.update(&ops)?;
        Ok(())
    }

    pub fn set_data<K: KvStore>(
        db: &mut K,
        id: ContractId,
        mut locator: ZkDataLocator,
        mut value: ZkScalar,
        size_diff: &mut u32,
    ) -> Result<ZkScalar, StateManagerError> {
        let contract_type = Self::type_of(db, id)?;
        let mut ops = Vec::new();

        if contract_type.locate(&locator)? != ZkStateModel::Scalar {
            return Err(StateManagerError::NonScalarLocatorError);
        }

        let prev_is_zero: bool = Self::get_data(db, id, &locator)?.is_zero().into();

        ops.push(if value.is_zero().into() {
            if !prev_is_zero {
                *size_diff -= 1;
            }
            WriteOp::Remove(keys::local_value(&id, &locator, true))
        } else {
            if prev_is_zero {
                *size_diff += 1;
            }
            WriteOp::Put(keys::local_value(&id, &locator, true), value.into())
        });

        while let Some(curr_loc) = locator.0.pop() {
            let curr_type = contract_type.locate(&locator)?;
            match curr_type.clone() {
                ZkStateModel::List {
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
                            } else if layer == log4_size - 1 {
                                let mut full_loc = locator.clone();
                                full_loc.0.push(leaf_index as u32);
                                Self::get_data(db, id, &full_loc)?
                            } else {
                                match db.get(keys::local_tree_aux(
                                    &id,
                                    &locator,
                                    aux_offset + leaf_index,
                                ))? {
                                    Some(b) => b.try_into()?,
                                    None => default_value,
                                }
                            });
                        }

                        value = H::hash(&dats);
                        default_value = H::hash(&[default_value; 4]);

                        curr_ind /= 4;

                        if layer > 0 {
                            let parent_aux_offset = ((1 << (2 * layer)) - 1) / 3;
                            let parent_index = parent_aux_offset + curr_ind;
                            let aux_key = keys::local_tree_aux(&id, &locator, parent_index);
                            ops.push(if value == default_value {
                                WriteOp::Remove(aux_key)
                            } else {
                                WriteOp::Put(aux_key, value.into())
                            });
                        }
                    }
                }
                ZkStateModel::Struct { field_types } => {
                    let mut dats = Vec::new();
                    for field_index in 0..field_types.len() {
                        dats.push(if field_index as u32 == curr_loc {
                            value
                        } else {
                            let mut full_loc = locator.clone();
                            full_loc.0.push(field_index as u32);
                            Self::get_data(db, id, &full_loc)?
                        });
                    }
                    value = H::hash(&dats);
                }
                ZkStateModel::Scalar => {
                    panic!()
                }
            }

            ops.push(if value == curr_type.compress_default::<H>() {
                WriteOp::Remove(keys::local_value(&id, &locator, false))
            } else {
                WriteOp::Put(keys::local_value(&id, &locator, false), value.into())
            });
        }

        db.update(&ops)?;
        Ok(value)
    }

    pub fn get_data<K: KvStore>(
        db: &K,
        cid: ContractId,
        locator: &ZkDataLocator,
    ) -> Result<ZkScalar, StateManagerError> {
        let sub_type = Self::type_of(db, cid)?.locate(locator)?;
        Ok(
            match db.get(keys::local_value(
                &cid,
                &locator,
                sub_type == ZkStateModel::Scalar,
            ))? {
                Some(b) => b.try_into()?,
                None => sub_type.compress_default::<H>(),
            },
        )
    }
}
