use super::*;

pub fn withdraw<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    contract_id: &ContractId,
    contract: &zk::ZkContract,
    withdraw_circuit_id: &u32,
    withdraws: &[ContractWithdraw],
    executor_fees: &mut Vec<Money>,
) -> Result<(zk::ZkVerifierKey, zk::ZkCompressedState), BlockchainError> {
    let withdraw_func = contract
        .withdraw_functions
        .get(*withdraw_circuit_id as usize)
        .ok_or(BlockchainError::ContractFunctionNotFound)?;
    let circuit = &withdraw_func.verifier_key;
    let state_model = zk::ZkStateModel::List {
        item_type: Box::new(zk::ZkStateModel::Struct {
            field_types: vec![
                zk::ZkStateModel::Scalar, // Enabled
                zk::ZkStateModel::Scalar, // Amount token-id
                zk::ZkStateModel::Scalar, // Amount
                zk::ZkStateModel::Scalar, // Fee token-id
                zk::ZkStateModel::Scalar, // Fee
                zk::ZkStateModel::Scalar, // Fingerprint
                zk::ZkStateModel::Scalar, // Calldata
            ],
        }),
        log4_size: withdraw_func.log4_payment_capacity,
    };
    let mut state_builder = zk::ZkStateBuilder::<CoreZkHasher>::new(state_model);
    for (i, withdraw) in withdraws.iter().enumerate() {
        if withdraw.contract_id != *contract_id
            || withdraw.withdraw_circuit_id != *withdraw_circuit_id
        {
            return Err(BlockchainError::DepositWithdrawPassedToWrongFunction);
        }
        let fingerprint = withdraw.fingerprint();
        executor_fees.push(withdraw.fee);
        state_builder.batch_set(&zk::ZkDeltaPairs(
            [
                (
                    zk::ZkDataLocator(vec![i as u64, 0]),
                    Some(zk::ZkScalar::from(1)),
                ),
                (
                    zk::ZkDataLocator(vec![i as u64, 1]),
                    Some(withdraw.amount.token_id.into()),
                ),
                (
                    zk::ZkDataLocator(vec![i as u64, 2]),
                    Some(zk::ZkScalar::from(withdraw.amount.amount)),
                ),
                (
                    zk::ZkDataLocator(vec![i as u64, 3]),
                    Some(withdraw.fee.token_id.into()),
                ),
                (
                    zk::ZkDataLocator(vec![i as u64, 4]),
                    Some(zk::ZkScalar::from(withdraw.fee.amount)),
                ),
                (zk::ZkDataLocator(vec![i as u64, 5]), Some(fingerprint)),
                (
                    zk::ZkDataLocator(vec![i as u64, 6]),
                    Some(withdraw.calldata),
                ),
            ]
            .into(),
        ))?;

        chain.apply_withdraw(withdraw)?;
    }
    let aux_data = state_builder.compress()?;
    Ok((circuit.clone(), aux_data))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{RamKvStore, WriteOp};

    #[test]
    fn test_empty_withdraw() {
        let chain = KvStoreChain::new(
            RamKvStore::new(),
            crate::config::blockchain::get_test_blockchain_config(),
        )
        .unwrap();

        let withdraw_state_model = zk::ZkStateModel::List {
            item_type: Box::new(zk::ZkStateModel::Struct {
                field_types: vec![
                    zk::ZkStateModel::Scalar, // Enabled
                    zk::ZkStateModel::Scalar, // Amount token-id
                    zk::ZkStateModel::Scalar, // Amount
                    zk::ZkStateModel::Scalar, // Fee token-id
                    zk::ZkStateModel::Scalar, // Fee
                    zk::ZkStateModel::Scalar, // Fingerprint
                    zk::ZkStateModel::Scalar, // Calldata
                ],
            }),
            log4_size: 1,
        };
        let empty_compressed =
            zk::ZkCompressedState::empty::<crate::core::ZkHasher>(withdraw_state_model);

        let (ops, ((_, aux_data), exec_fees)) = chain
            .isolated(|chain| {
                let mut exec_fees = Vec::new();
                let contract_id = chain.config.mpn_config.mpn_contract_id;
                let contract = chain.get_contract(contract_id)?;
                Ok((
                    withdraw(chain, &contract_id, &contract, &0, &[], &mut exec_fees)?,
                    exec_fees,
                ))
            })
            .unwrap();

        assert_eq!(aux_data, empty_compressed);
        assert!(ops.is_empty());
        assert!(exec_fees.is_empty());
    }

    #[test]
    fn test_single_withdraw() {
        let mut chain = KvStoreChain::new(
            RamKvStore::new(),
            crate::config::blockchain::get_test_blockchain_config(),
        )
        .unwrap();

        let contract_id = chain.config.mpn_config.mpn_contract_id;

        let abc = TxBuilder::new(&Vec::from("ABC"));
        let mut cont_deposit = ContractDeposit {
            memo: "".into(),
            src: abc.get_address(),
            contract_id,
            deposit_circuit_id: 0,
            calldata: zk::ZkScalar::from(888),
            nonce: 1,
            amount: Money::ziesha(1000),
            fee: Money::ziesha(0),
            sig: None,
        };
        abc.sign_deposit(&mut cont_deposit);
        chain.apply_deposit(&cont_deposit).unwrap();

        let cont_withdraw = ContractWithdraw {
            memo: "".into(),
            contract_id,
            withdraw_circuit_id: 0,
            calldata: zk::ZkScalar::from(777),
            dst: abc.get_address(),
            amount: Money::ziesha(200),
            fee: Money::ziesha(50),
        };

        let (ops, ((_, aux_data), exec_fees)) = chain
            .isolated(|chain| {
                let mut exec_fees = Vec::new();
                let contract = chain.get_contract(contract_id)?;
                Ok((
                    withdraw(
                        chain,
                        &contract_id,
                        &contract,
                        &0,
                        &[cont_withdraw.clone()],
                        &mut exec_fees,
                    )?,
                    exec_fees,
                ))
            })
            .unwrap();
        let empty_leaf =
            <crate::core::ZkHasher as crate::zk::ZkHasher>::hash(&[zk::ZkScalar::from(0); 7]);

        let expected_hash = <crate::core::ZkHasher as crate::zk::ZkHasher>::hash(&[
            <crate::core::ZkHasher as crate::zk::ZkHasher>::hash(&[
                zk::ZkScalar::from(1),
                zk::ZkScalar::from(1),
                zk::ZkScalar::from(200),
                zk::ZkScalar::from(1),
                zk::ZkScalar::from(50),
                zk::ZkScalar::from(cont_withdraw.fingerprint()),
                zk::ZkScalar::from(777),
            ]),
            empty_leaf,
            empty_leaf,
            empty_leaf,
        ]);

        let expected_aux = zk::ZkCompressedState {
            state_hash: expected_hash,
            state_size: 7,
        };

        assert_eq!(aux_data, expected_aux);

        let expected_ops = vec![
            WriteOp::Put(
                "ACB-ed8c19c6a4cf1460e961f7bae8eea54d437b9edac27cbeb09be32ae367adf9098a-Ziesha"
                    .into(),
                Amount(9200).into(),
            ),
            WriteOp::Put(
                "CAB-341b882a2d0429ed82ef0717eb9dcfb39012421173979bc4b550d291da91ac3e-Ziesha"
                    .into(),
                Amount(750).into(),
            ),
        ];

        assert_eq!(ops, expected_ops);
        assert_eq!(
            exec_fees,
            vec![Money {
                token_id: TokenId::Ziesha,
                amount: Amount(50)
            }]
        );
    }

    #[test]
    fn test_custom_token_withdraw() {
        let mut chain = KvStoreChain::new(
            RamKvStore::new(),
            crate::config::blockchain::get_test_blockchain_config(),
        )
        .unwrap();

        let contract_id = chain.config.mpn_config.mpn_contract_id;

        let abc = TxBuilder::new(&Vec::from("ABC"));
        let (tx_delta, kiwi_token_id) = abc.create_token(
            "".into(),
            "KeyvanCoin".into(),
            "KIWI".into(),
            Amount(100000),
            3,
            None,
            Money::ziesha(0),
            1,
        );
        chain.apply_tx(&tx_delta.tx, false).unwrap();

        let mut cont_deposit_ziesha = ContractDeposit {
            memo: "".into(),
            src: abc.get_address(),
            contract_id,
            deposit_circuit_id: 0,
            calldata: zk::ZkScalar::from(888),
            nonce: 1,
            amount: Money::ziesha(1000),
            fee: Money::ziesha(0),
            sig: None,
        };
        abc.sign_deposit(&mut cont_deposit_ziesha);
        chain.apply_deposit(&cont_deposit_ziesha).unwrap();

        let mut cont_deposit_kiwi = ContractDeposit {
            memo: "".into(),
            src: abc.get_address(),
            contract_id,
            deposit_circuit_id: 0,
            calldata: zk::ZkScalar::from(888),
            nonce: 2,
            amount: Money {
                token_id: kiwi_token_id,
                amount: Amount(1000),
            },
            fee: Money::ziesha(0),
            sig: None,
        };
        abc.sign_deposit(&mut cont_deposit_kiwi);
        chain.apply_deposit(&cont_deposit_kiwi).unwrap();

        let cont_withdraw = ContractWithdraw {
            memo: "".into(),
            contract_id,
            withdraw_circuit_id: 0,
            calldata: zk::ZkScalar::from(777),
            dst: abc.get_address(),
            amount: Money {
                token_id: kiwi_token_id,
                amount: Amount(333),
            },
            fee: Money::ziesha(444),
        };

        let (ops, ((_, aux_data), exec_fees)) = chain
            .isolated(|chain| {
                let mut exec_fees = Vec::new();
                let contract = chain.get_contract(contract_id)?;
                Ok((
                    withdraw(
                        chain,
                        &contract_id,
                        &contract,
                        &0,
                        &[cont_withdraw.clone()],
                        &mut exec_fees,
                    )?,
                    exec_fees,
                ))
            })
            .unwrap();
        let empty_leaf =
            <crate::core::ZkHasher as crate::zk::ZkHasher>::hash(&[zk::ZkScalar::from(0); 7]);

        let expected_hash = <crate::core::ZkHasher as crate::zk::ZkHasher>::hash(&[
            <crate::core::ZkHasher as crate::zk::ZkHasher>::hash(&[
                zk::ZkScalar::from(1),
                zk::ZkScalar::from(kiwi_token_id),
                zk::ZkScalar::from(333),
                zk::ZkScalar::from(1),
                zk::ZkScalar::from(444),
                zk::ZkScalar::from(cont_withdraw.fingerprint()),
                zk::ZkScalar::from(777),
            ]),
            empty_leaf,
            empty_leaf,
            empty_leaf,
        ]);

        let expected_aux = zk::ZkCompressedState {
            state_hash: expected_hash,
            state_size: 7,
        };

        assert_eq!(aux_data, expected_aux);

        let expected_ops = vec![
            WriteOp::Put(
                "ACB-ed8c19c6a4cf1460e961f7bae8eea54d437b9edac27cbeb09be32ae367adf9098a-0x214bfab03425c710055132a7e6656d61442f65e56d2efd930b4f090eb58f952c"
                    .into(),
                Amount(99333).into(),
            ),
            WriteOp::Put(
                "CAB-341b882a2d0429ed82ef0717eb9dcfb39012421173979bc4b550d291da91ac3e-0x214bfab03425c710055132a7e6656d61442f65e56d2efd930b4f090eb58f952c"
                    .into(),
                Amount(667).into(),
            ),
            WriteOp::Put(
                "CAB-341b882a2d0429ed82ef0717eb9dcfb39012421173979bc4b550d291da91ac3e-Ziesha"
                    .into(),
                Amount(556).into(),
            )
        ];

        assert_eq!(ops, expected_ops);
        assert_eq!(
            exec_fees,
            vec![Money {
                token_id: TokenId::Ziesha,
                amount: Amount(444)
            }]
        );
    }
}
