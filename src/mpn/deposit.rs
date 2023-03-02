use super::*;
use crate::blockchain::BlockchainError;
use crate::core::{ContractId, ZkHasher};
use crate::db::{keys, KvStore, WriteOp};
use crate::zk::{
    KvStoreStateManager, ZkCompressedState, ZkDataLocator, ZkDeltaPairs, ZkScalar, ZkStateBuilder,
    ZkStateModel,
};
use std::collections::HashSet;

pub fn deposit<K: KvStore>(
    mpn_contract_id: ContractId,
    mpn_log4_account_capacity: u8,
    db: &mut K,
    txs: &[MpnDeposit],
) -> Result<(ZkPublicInputs, Vec<DepositTransition>), BlockchainError> {
    let mut mirror = db.mirror();

    let mut transitions = Vec::new();
    let mut rejected = Vec::new();
    let mut accepted = Vec::new();
    let height = KvStoreStateManager::<ZkHasher>::height_of(db, mpn_contract_id).unwrap();
    let root = KvStoreStateManager::<ZkHasher>::root(db, mpn_contract_id).unwrap();

    let state = root.state_hash;
    let mut state_size = root.state_size;
    let mut rejected_pub_keys = HashSet::new();

    for tx in txs.into_iter() {
        if transitions.len() == 1 << (2 * LOG4_DEPOSIT_BATCH_SIZE) {
            break;
        }
        let acc = KvStoreStateManager::<ZkHasher>::get_mpn_account(
            &mirror,
            mpn_contract_id,
            tx.zk_address_index(mpn_log4_account_capacity),
        )
        .unwrap();
        let acc_token = acc.tokens.get(&tx.zk_token_index).clone();
        let src_pub = tx.payment.src.clone();
        if rejected_pub_keys.contains(&src_pub)
            || (acc.address != Default::default() && tx.zk_address.0.decompress() != acc.address)
            || (acc_token.is_some() && acc_token.unwrap().token_id != tx.payment.amount.token_id)
        {
            rejected.push(tx.clone());
            rejected_pub_keys.insert(src_pub);
            continue;
        } else {
            let mut updated_acc = MpnAccount {
                address: tx.zk_address.0.decompress(),
                tokens: acc.tokens.clone(),
                nonce: acc.nonce,
            };
            updated_acc
                .tokens
                .entry(tx.zk_token_index)
                .or_insert(Money::new(tx.payment.amount.token_id, 0))
                .amount += tx.payment.amount.amount;

            let balance_proof = KvStoreStateManager::<ZkHasher>::prove(
                &mirror,
                mpn_contract_id,
                ZkDataLocator(vec![tx.zk_address_index(mpn_log4_account_capacity), 3]),
                tx.zk_token_index,
            )
            .unwrap();
            let proof = KvStoreStateManager::<ZkHasher>::prove(
                &mirror,
                mpn_contract_id,
                ZkDataLocator(vec![]),
                tx.zk_address_index(mpn_log4_account_capacity),
            )
            .unwrap();

            KvStoreStateManager::<ZkHasher>::set_mpn_account(
                &mut mirror,
                mpn_contract_id,
                tx.zk_address_index(mpn_log4_account_capacity),
                updated_acc,
                &mut state_size,
            )
            .unwrap();

            transitions.push(DepositTransition {
                tx: tx.clone(),
                before: acc.clone(),
                before_balances_hash: acc.tokens_hash::<ZkHasher>(LOG4_TOKENS_TREE_SIZE),
                before_balance: acc_token.cloned().unwrap_or_default(),
                proof,
                balance_proof,
            });
            accepted.push(tx);
        }
    }
    let next_state =
        KvStoreStateManager::<ZkHasher>::get_data(&mirror, mpn_contract_id, &ZkDataLocator(vec![]))
            .unwrap();
    mirror
        .update(&[WriteOp::Put(
            keys::local_root(&mpn_contract_id),
            ZkCompressedState {
                state_hash: next_state,
                state_size,
            }
            .into(),
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
        log4_size: LOG4_DEPOSIT_BATCH_SIZE as u8,
    };
    let mut state_builder = ZkStateBuilder::<ZkHasher>::new(state_model.clone());

    for (i, trans) in transitions.iter().enumerate() {
        use crate::zk::ZkHasher;
        let calldata = crate::core::ZkHasher::hash(&[
            ZkScalar::from(trans.tx.zk_address.0.decompress().0),
            ZkScalar::from(trans.tx.zk_address.0.decompress().1),
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
    db.update(&ops)?;

    Ok((
        ZkPublicInputs {
            height,
            state,
            aux_data,
            next_state,
        },
        transitions,
    ))
}
