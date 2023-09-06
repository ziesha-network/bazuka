use super::*;
use crate::blockchain::{Blockchain, BlockchainError};
use crate::core::{ContractId, ZkHasher};
use crate::db::{keys, KvStore, WriteOp};
use crate::zk::{
    KvStoreStateManager, ZkCompressedState, ZkDataLocator, ZkDeltaPairs, ZkScalar, ZkStateBuilder,
    ZkStateModel,
};
use std::collections::HashSet;

pub fn deposit<K: KvStore, B: Blockchain<K>>(
    mpn_contract_id: ContractId,
    mpn_log4_account_capacity: u8,
    log4_token_tree_size: u8,
    log4_batch_size: u8,
    db: &mut B,
    txs: &[MpnDeposit],
    new_account_indices: &mut HashMap<MpnAddress, u64>,
    check_balance: bool,
) -> Result<(ZkCompressedState, ZkPublicInputs, Vec<DepositTransition>), BlockchainError> {
    let mut mirror = db.database().mirror();

    let mut transitions = Vec::new();
    let mut rejected = Vec::new();
    let mut accepted = Vec::new();
    let height = KvStoreStateManager::<ZkHasher>::height_of(&mirror, mpn_contract_id).unwrap();
    let root = KvStoreStateManager::<ZkHasher>::root(&mirror, mpn_contract_id).unwrap();

    let mpn_account_count = db.get_mpn_account_count()?;

    let state = root.state_hash;
    let mut state_size = root.state_size;
    let mut rejected_pub_keys = HashSet::new();

    for tx in txs.into_iter() {
        if transitions.len() == 1 << (2 * log4_batch_size) {
            break;
        }

        let mpn_addr = MpnAddress {
            pub_key: tx.mpn_address.clone(),
        };
        let mut new_index = None;
        let account_index = if let Some(ind) = db.get_mpn_account_indices(mpn_addr.clone())?.first()
        {
            *ind
        } else {
            if let Some(ind) = new_account_indices.get(&mpn_addr) {
                *ind
            } else {
                let ind = mpn_account_count + new_account_indices.len() as u64;
                new_index = Some(ind);
                ind
            }
        };

        let acc = KvStoreStateManager::<ZkHasher>::get_mpn_account(
            &mirror,
            mpn_contract_id,
            account_index,
        )
        .unwrap();
        let src_pub = tx.payment.src.clone();
        let zk_token_index = if let Some(ind) =
            acc.find_token_index(mpn_log4_account_capacity, tx.payment.amount.token_id, true)
        {
            ind
        } else {
            rejected.push(tx.clone());
            rejected_pub_keys.insert(src_pub);
            continue;
        };
        let acc_token = acc.tokens.get(&zk_token_index).clone();

        let mut isolated = mirror.mirror();
        let mut isolated_state_size = state_size;
        if rejected_pub_keys.contains(&src_pub)
            || (acc.address != Default::default() && tx.mpn_address.0.decompress() != acc.address)
            || (acc_token.is_some() && acc_token.unwrap().token_id != tx.payment.amount.token_id)
        {
            rejected.push(tx.clone());
            rejected_pub_keys.insert(src_pub);
            continue;
        } else {
            if check_balance {
                // Check src have enough balance
                let src_amount_bal = db.get_balance(src_pub.clone(), tx.payment.amount.token_id)?;
                if src_amount_bal < tx.payment.amount.amount {
                    rejected.push(tx.clone());
                    rejected_pub_keys.insert(src_pub);
                    continue;
                }
                isolated.update(&[WriteOp::Put(
                    crate::db::keys::account_balance(&src_pub, tx.payment.amount.token_id),
                    (src_amount_bal - tx.payment.amount.amount).into(),
                )])?;
                let src_fee_bal = db.get_balance(src_pub.clone(), tx.payment.fee.token_id)?;
                if src_fee_bal < tx.payment.fee.amount {
                    rejected.push(tx.clone());
                    rejected_pub_keys.insert(src_pub);
                    continue;
                }
                isolated.update(&[WriteOp::Put(
                    crate::db::keys::account_balance(&src_pub, tx.payment.fee.token_id),
                    (src_fee_bal - tx.payment.fee.amount).into(),
                )])?;
            }

            let mut updated_acc = MpnAccount {
                address: tx.mpn_address.0.decompress(),
                tokens: acc.tokens.clone(),
                withdraw_nonce: acc.withdraw_nonce,
                tx_nonce: acc.tx_nonce,
            };
            updated_acc
                .tokens
                .entry(zk_token_index)
                .or_insert(Money::new(tx.payment.amount.token_id, 0))
                .amount += tx.payment.amount.amount;

            let balance_proof = KvStoreStateManager::<ZkHasher>::prove(
                &isolated,
                mpn_contract_id,
                ZkDataLocator(vec![account_index, 4]),
                zk_token_index,
            )
            .unwrap();
            let proof = KvStoreStateManager::<ZkHasher>::prove(
                &isolated,
                mpn_contract_id,
                ZkDataLocator(vec![]),
                account_index,
            )
            .unwrap();

            KvStoreStateManager::<ZkHasher>::set_mpn_account(
                &mut isolated,
                mpn_contract_id,
                account_index,
                updated_acc,
                &mut isolated_state_size,
            )
            .unwrap();

            if let Some(new_ind) = new_index {
                new_account_indices.insert(mpn_addr.clone(), new_ind);
            }
            transitions.push(DepositTransition {
                enabled: true,
                tx: tx.clone(),
                account_index,
                token_index: zk_token_index,
                before: acc.clone(),
                before_balances_hash: acc.tokens_hash::<ZkHasher>(log4_token_tree_size),
                before_balance: acc_token.cloned().unwrap_or_default(),
                proof,
                balance_proof,
            });
            accepted.push(tx);
            mirror.update(&isolated.to_ops())?;
            state_size = isolated_state_size;
        }
    }
    let next_state =
        KvStoreStateManager::<ZkHasher>::get_data(&mirror, mpn_contract_id, &ZkDataLocator(vec![]))
            .unwrap();
    let new_root = ZkCompressedState {
        state_hash: next_state,
        state_size,
    };
    mirror
        .update(&[WriteOp::Put(
            keys::local_root(&mpn_contract_id),
            new_root.clone().into(),
        )])
        .unwrap();

    let state_model = ZkStateModel::List {
        item_type: Box::new(ZkStateModel::Struct {
            field_types: vec![
                ZkStateModel::Scalar, // Enabled
                ZkStateModel::Scalar, // Token-id
                ZkStateModel::Scalar, // Amount
                ZkStateModel::Scalar, // Calldata
            ],
        }),
        log4_size: log4_batch_size,
    };
    let mut state_builder = ZkStateBuilder::<ZkHasher>::new(state_model.clone());

    for (i, trans) in transitions.iter().enumerate() {
        use crate::zk::ZkHasher;
        let calldata = crate::core::ZkHasher::hash(&[
            ZkScalar::from(trans.tx.mpn_address.0.decompress().0),
            ZkScalar::from(trans.tx.mpn_address.0.decompress().1),
        ]);
        state_builder
            .batch_set(&ZkDeltaPairs(
                [
                    (ZkDataLocator(vec![i as u64, 0]), Some(ZkScalar::from(1))),
                    (
                        ZkDataLocator(vec![i as u64, 1]),
                        Some(trans.tx.payment.amount.token_id.into()),
                    ),
                    (
                        ZkDataLocator(vec![i as u64, 2]),
                        Some(trans.tx.payment.amount.amount.into()),
                    ),
                    (
                        ZkDataLocator(vec![i as u64, 3]),
                        Some(ZkScalar::from(calldata)),
                    ),
                ]
                .into(),
            ))
            .unwrap();
    }
    let aux_data = state_builder.compress().unwrap().state_hash;

    let ops = mirror.to_ops();
    db.database_mut().update(&ops)?;

    Ok((
        new_root,
        ZkPublicInputs {
            height,
            state,
            aux_data,
            next_state,
        },
        transitions,
    ))
}
