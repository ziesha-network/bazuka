use super::*;
use crate::blockchain::BlockchainError;
use crate::core::{ContractId, TokenId, ZkHasher};
use crate::db::{keys, KvStore, WriteOp};
use crate::zk::{KvStoreStateManager, ZkCompressedState, ZkDataLocator, ZkScalar};
use rayon::prelude::*;

pub fn update<K: KvStore>(
    mpn_contract_id: ContractId,
    mpn_log4_account_capacity: u8,
    log4_token_tree_size: u8,
    log4_batch_size: u8,
    fee_token: TokenId,
    db: &mut K,
    txs: &[MpnTransaction],
) -> Result<(ZkCompressedState, ZkPublicInputs, Vec<UpdateTransition>), BlockchainError> {
    let mut rejected = Vec::new();
    let mut accepted = Vec::new();
    let mut transitions = Vec::new();

    let root = KvStoreStateManager::<ZkHasher>::root(db, mpn_contract_id).unwrap();
    let height = KvStoreStateManager::<ZkHasher>::height_of(db, mpn_contract_id).unwrap();

    let state = root.state_hash;
    let mut state_size = root.state_size;

    let mut mirror = db.mirror();

    let txs = txs
        .into_par_iter()
        .filter(|tx| {
            tx.fee.token_id == fee_token
                && tx.src_pub_key.is_on_curve()
                && tx.dst_pub_key.is_on_curve()
        })
        .collect::<Vec<_>>();

    for tx in txs.into_iter() {
        if transitions.len() == 1 << (2 * log4_batch_size) {
            break;
        }
        let src_before = KvStoreStateManager::<ZkHasher>::get_mpn_account(
            &mirror,
            mpn_contract_id,
            tx.src_index(mpn_log4_account_capacity),
        )
        .unwrap();
        let dst_before = KvStoreStateManager::<ZkHasher>::get_mpn_account(
            &mirror,
            mpn_contract_id,
            tx.dst_index(mpn_log4_account_capacity),
        )
        .unwrap();
        let src_token = if let Some(src_token) = src_before.tokens.get(&tx.src_token_index) {
            src_token.clone()
        } else {
            rejected.push(tx.clone());
            continue;
        };
        let dst_token = dst_before.tokens.get(&tx.dst_token_index);
        if tx.nonce != src_before.nonce
            || src_before.address != tx.src_pub_key.decompress()
            || (dst_before.address.is_on_curve()
                && dst_before.address != tx.dst_pub_key.decompress())
            || dst_token.is_some() && (src_token.token_id != dst_token.unwrap().token_id)
            || src_token.token_id != tx.amount.token_id
            || src_token.amount < tx.amount.amount
        {
            rejected.push(tx.clone());
            continue;
        } else {
            let src_proof = KvStoreStateManager::<ZkHasher>::prove(
                &mirror,
                mpn_contract_id,
                ZkDataLocator(vec![]),
                tx.src_index(mpn_log4_account_capacity),
            )
            .unwrap();

            let mut src_after = MpnAccount {
                address: src_before.address.clone(),
                tokens: src_before.tokens.clone(),
                nonce: src_before.nonce + 1,
            };

            let src_balance_proof = KvStoreStateManager::<ZkHasher>::prove(
                &mirror,
                mpn_contract_id,
                ZkDataLocator(vec![tx.src_index(mpn_log4_account_capacity), 3]),
                tx.src_token_index,
            )
            .unwrap();

            src_after
                .tokens
                .get_mut(&tx.src_token_index)
                .unwrap()
                .amount -= tx.amount.amount;
            KvStoreStateManager::<ZkHasher>::set_mpn_account(
                &mut mirror,
                mpn_contract_id,
                tx.src_index(mpn_log4_account_capacity),
                src_after.clone(),
                &mut state_size,
            )
            .unwrap();

            let src_fee_token =
                if let Some(src_fee_token) = src_after.tokens.get(&tx.src_fee_token_index) {
                    src_fee_token.clone()
                } else {
                    rejected.push(tx.clone());
                    continue;
                };

            if src_fee_token.token_id != tx.fee.token_id || src_fee_token.amount < tx.fee.amount {
                rejected.push(tx.clone());
                continue;
            }

            let src_fee_balance_proof = KvStoreStateManager::<ZkHasher>::prove(
                &mirror,
                mpn_contract_id,
                ZkDataLocator(vec![tx.src_index(mpn_log4_account_capacity), 3]),
                tx.src_fee_token_index,
            )
            .unwrap();

            src_after
                .tokens
                .get_mut(&tx.src_fee_token_index)
                .unwrap()
                .amount -= tx.fee.amount;
            KvStoreStateManager::<ZkHasher>::set_mpn_account(
                &mut mirror,
                mpn_contract_id,
                tx.src_index(mpn_log4_account_capacity),
                src_after,
                &mut state_size,
            )
            .unwrap();

            let dst_proof = KvStoreStateManager::<ZkHasher>::prove(
                &mirror,
                mpn_contract_id,
                ZkDataLocator(vec![]),
                tx.dst_index(mpn_log4_account_capacity),
            )
            .unwrap();
            let dst_balance_proof = KvStoreStateManager::<ZkHasher>::prove(
                &mirror,
                mpn_contract_id,
                ZkDataLocator(vec![tx.dst_index(mpn_log4_account_capacity), 3]),
                tx.dst_token_index,
            )
            .unwrap();

            let dst_before = KvStoreStateManager::<ZkHasher>::get_mpn_account(
                &mirror,
                mpn_contract_id,
                tx.dst_index(mpn_log4_account_capacity),
            )
            .unwrap();
            let dst_token = dst_before.tokens.get(&tx.dst_token_index);

            let mut dst_after = MpnAccount {
                address: tx.dst_pub_key.0.decompress(),
                tokens: dst_before.tokens.clone(),
                nonce: dst_before.nonce,
            };
            dst_after
                .tokens
                .entry(tx.dst_token_index)
                .or_insert(Money::new(tx.amount.token_id, 0))
                .amount += tx.amount.amount;
            KvStoreStateManager::<ZkHasher>::set_mpn_account(
                &mut mirror,
                mpn_contract_id,
                tx.dst_index(mpn_log4_account_capacity),
                dst_after,
                &mut state_size,
            )
            .unwrap();

            transitions.push(UpdateTransition {
                tx: tx.clone(),
                src_before: src_before.clone(),
                src_proof,
                dst_before: dst_before.clone(),
                dst_proof,
                src_balance_proof,
                src_fee_balance_proof,
                dst_balance_proof,
                src_before_balance: src_token.clone(),
                src_before_fee_balance: src_fee_token.clone(),
                dst_before_balance: dst_token.cloned().unwrap_or(Money::default()),
                src_before_balances_hash: src_before.tokens_hash::<ZkHasher>(log4_token_tree_size),
                dst_before_balances_hash: dst_before.tokens_hash::<ZkHasher>(log4_token_tree_size),
            });
            accepted.push(tx);
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

    let aux_data = {
        use crate::zk::ZkHasher;
        crate::core::ZkHasher::hash(&[
            fee_token.into(),
            ZkScalar::from(
                accepted
                    .iter()
                    .map(|tx| Into::<u64>::into(tx.fee.amount))
                    .sum::<u64>(),
            ),
        ])
    };

    let ops = mirror.to_ops();
    db.update(&ops)?;
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
