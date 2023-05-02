use super::*;
use crate::blockchain::{Blockchain, BlockchainError};
use crate::core::{ContractId, ZkHasher};
use crate::db::{keys, KvStore, WriteOp};
use crate::zk::{
    KvStoreStateManager, ZkCompressedState, ZkDataLocator, ZkDeltaPairs, ZkScalar, ZkStateBuilder,
    ZkStateModel,
};

pub fn withdraw<K: KvStore, B: Blockchain<K>>(
    mpn_contract_id: ContractId,
    mpn_log4_account_capacity: u8,
    log4_token_tree_size: u8,
    log4_batch_size: u8,
    db: &mut B,
    txs: &[MpnWithdraw],
    new_account_indices: &HashMap<MpnAddress, u64>,
) -> Result<(ZkCompressedState, ZkPublicInputs, Vec<WithdrawTransition>), BlockchainError> {
    let mut mirror = db.database().mirror();

    let mut transitions = Vec::new();
    let mut rejected = Vec::new();
    let mut accepted = Vec::new();
    let height = KvStoreStateManager::<ZkHasher>::height_of(&mirror, mpn_contract_id).unwrap();
    let root = KvStoreStateManager::<ZkHasher>::root(&mirror, mpn_contract_id).unwrap();

    let state = root.state_hash;
    let mut state_size = root.state_size;

    for tx in txs.into_iter() {
        if transitions.len() == 1 << (2 * log4_batch_size) {
            break;
        }

        let mpn_addr = MpnAddress {
            pub_key: tx.zk_address.clone(),
        };
        let account_index = if let Some(ind) = db.get_mpn_account_indices(mpn_addr.clone())?.first()
        {
            *ind
        } else {
            if let Some(ind) = new_account_indices.get(&mpn_addr) {
                *ind
            } else {
                rejected.push(tx.clone());
                continue;
            }
        };

        let acc = KvStoreStateManager::<ZkHasher>::get_mpn_account(
            &mirror,
            mpn_contract_id,
            account_index,
        )
        .unwrap();

        let (zk_token_index, zk_fee_token_index) = if let Some((ind, fee_ind)) = acc
            .find_token_index(mpn_log4_account_capacity, tx.payment.amount.token_id, false)
            .zip(acc.find_token_index(mpn_log4_account_capacity, tx.payment.fee.token_id, false))
        {
            (ind, fee_ind)
        } else {
            rejected.push(tx.clone());
            continue;
        };

        let acc_token = if let Some(acc_token) = acc.tokens.get(&zk_token_index) {
            acc_token.clone()
        } else {
            rejected.push(tx.clone());
            continue;
        };

        if (acc.address != Default::default() && tx.zk_address.0.decompress() != acc.address)
            || !tx.verify_calldata::<ZkHasher>()
            || !tx.verify_signature::<ZkHasher>()
            || tx.zk_nonce != acc.withdraw_nonce + 1
            || tx.payment.amount.token_id != acc_token.token_id
            || tx.payment.amount.amount > acc_token.amount
        {
            println!("{} {}", tx.zk_nonce, acc.withdraw_nonce);
            rejected.push(tx.clone());
            continue;
        } else {
            let mut updated_acc = MpnAccount {
                address: tx.zk_address.0.decompress(),
                tokens: acc.tokens.clone(),
                tx_nonce: acc.tx_nonce,
                withdraw_nonce: acc.withdraw_nonce + 1,
            };

            let before_token_hash = updated_acc.tokens_hash::<ZkHasher>(log4_token_tree_size);
            let token_balance_proof = KvStoreStateManager::<ZkHasher>::prove(
                &mirror,
                mpn_contract_id,
                ZkDataLocator(vec![account_index, 4]),
                zk_token_index,
            )
            .unwrap();

            updated_acc.tokens.get_mut(&zk_token_index).unwrap().amount -= tx.payment.amount.amount;
            KvStoreStateManager::<ZkHasher>::set_mpn_account(
                &mut mirror,
                mpn_contract_id,
                account_index,
                updated_acc.clone(),
                &mut state_size,
            )
            .unwrap();

            let fee_balance_proof = KvStoreStateManager::<ZkHasher>::prove(
                &mirror,
                mpn_contract_id,
                ZkDataLocator(vec![account_index, 4]),
                zk_fee_token_index,
            )
            .unwrap();

            let acc_fee_token =
                if let Some(src_fee_token) = updated_acc.tokens.get(&zk_fee_token_index) {
                    src_fee_token.clone()
                } else {
                    rejected.push(tx.clone());
                    continue;
                };
            if tx.payment.fee.token_id != acc_fee_token.token_id
                || tx.payment.fee.amount > acc_fee_token.amount
            {
                rejected.push(tx.clone());
                continue;
            }

            updated_acc
                .tokens
                .get_mut(&zk_fee_token_index)
                .unwrap()
                .amount -= tx.payment.fee.amount;

            let proof = KvStoreStateManager::<ZkHasher>::prove(
                &mirror,
                mpn_contract_id,
                ZkDataLocator(vec![]),
                account_index,
            )
            .unwrap();

            KvStoreStateManager::<ZkHasher>::set_mpn_account(
                &mut mirror,
                mpn_contract_id,
                account_index,
                updated_acc,
                &mut state_size,
            )
            .unwrap();

            transitions.push(WithdrawTransition {
                enabled: true,
                account_index,
                token_index: zk_token_index,
                fee_token_index: zk_fee_token_index,
                tx: tx.clone(),
                before: acc,
                before_token_balance: acc_token.clone(),
                before_fee_balance: acc_fee_token.clone(),
                proof,
                token_balance_proof,
                fee_balance_proof,
                before_token_hash,
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

    let state_model = ZkStateModel::List {
        item_type: Box::new(ZkStateModel::Struct {
            field_types: vec![
                ZkStateModel::Scalar, // Enabled
                ZkStateModel::Scalar, // Amount
                ZkStateModel::Scalar, // Amount token-id
                ZkStateModel::Scalar, // Fee
                ZkStateModel::Scalar, // Fee token-id
                ZkStateModel::Scalar, // Fingerprint
                ZkStateModel::Scalar, // Calldata
            ],
        }),
        log4_size: log4_batch_size,
    };
    let mut state_builder = ZkStateBuilder::<crate::core::ZkHasher>::new(state_model.clone());
    for (i, trans) in transitions.iter().enumerate() {
        use crate::zk::ZkHasher;
        let calldata = crate::core::ZkHasher::hash(&[
            ZkScalar::from(trans.tx.zk_address.0.decompress().0),
            ZkScalar::from(trans.tx.zk_address.0.decompress().1),
            ZkScalar::from(trans.tx.zk_nonce as u64),
            ZkScalar::from(trans.tx.zk_sig.r.0),
            ZkScalar::from(trans.tx.zk_sig.r.1),
            ZkScalar::from(trans.tx.zk_sig.s),
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
                        Some(trans.tx.payment.fee.token_id.into()),
                    ),
                    (
                        ZkDataLocator(vec![i as u64, 4]),
                        Some(trans.tx.payment.fee.amount.into()),
                    ),
                    (
                        ZkDataLocator(vec![i as u64, 5]),
                        Some(trans.tx.payment.fingerprint()),
                    ),
                    (ZkDataLocator(vec![i as u64, 6]), Some(calldata)),
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

#[cfg(test)]
use super::tests::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::TxBuilder;

    #[test]
    fn test_withdraw_empty() {
        let chain_conf = crate::config::blockchain::get_blockchain_config();
        let conf = chain_conf.mpn_config.clone();
        let mut db = fresh_db(chain_conf);
        withdraw(
            conf.mpn_contract_id,
            conf.log4_tree_size,
            conf.log4_token_tree_size,
            conf.log4_withdraw_batch_size,
            &mut db,
            &[],
            &Default::default(),
        )
        .unwrap();
    }

    #[test]
    fn test_withdraw() {
        let mut new_account_indices = HashMap::new();
        let chain_conf = crate::config::blockchain::get_blockchain_config();
        let conf = chain_conf.mpn_config.clone();
        let mut db = fresh_db(chain_conf);
        let abc = TxBuilder::new(&Vec::from("ABC"));
        let initial_dep = abc.deposit_mpn(
            "".into(),
            conf.mpn_contract_id,
            abc.get_mpn_address(),
            1,
            Money {
                amount: Amount(10056),
                token_id: TokenId::Custom(123.into()),
            },
            Money::ziesha(0),
        );

        // An initial amount to be withdrawn
        assert_eq!(
            deposit::deposit(
                conf.mpn_contract_id,
                conf.log4_tree_size,
                conf.log4_token_tree_size,
                conf.log4_deposit_batch_size,
                &mut db,
                &[initial_dep],
                &mut new_account_indices
            )
            .unwrap()
            .2
            .len(),
            1
        );

        let withdrawal = abc.withdraw_mpn(
            "".into(),
            conf.mpn_contract_id,
            1,
            Money {
                amount: Amount(30),
                token_id: TokenId::Custom(123.into()),
            },
            Money {
                amount: Amount(26),
                token_id: TokenId::Custom(123.into()),
            },
            abc.get_address(),
        );

        assert_eq!(
            withdraw(
                conf.mpn_contract_id,
                conf.log4_tree_size,
                conf.log4_token_tree_size,
                conf.log4_withdraw_batch_size,
                &mut db,
                &[withdrawal],
                &new_account_indices,
            )
            .unwrap()
            .2
            .len(),
            1
        );
    }
}
