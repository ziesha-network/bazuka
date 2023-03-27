use super::*;

pub fn deposit<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    contract_id: &ContractId,
    contract: &zk::ZkContract,
    deposit_circuit_id: &u32,
    deposits: &[ContractDeposit],
    executor_fees: &mut Vec<Money>,
) -> Result<(zk::ZkVerifierKey, zk::ZkCompressedState), BlockchainError> {
    let deposit_func = contract
        .deposit_functions
        .get(*deposit_circuit_id as usize)
        .ok_or(BlockchainError::ContractFunctionNotFound)?;
    let circuit = &deposit_func.verifier_key;
    let state_model = zk::ZkStateModel::List {
        item_type: Box::new(zk::ZkStateModel::Struct {
            field_types: vec![
                zk::ZkStateModel::Scalar, // Enabled
                zk::ZkStateModel::Scalar, // Token-id
                zk::ZkStateModel::Scalar, // Amount
                zk::ZkStateModel::Scalar, // Calldata
            ],
        }),
        log4_size: deposit_func.log4_payment_capacity,
    };
    let mut state_builder = zk::ZkStateBuilder::<CoreZkHasher>::new(state_model);
    for (i, deposit) in deposits.iter().enumerate() {
        if deposit.contract_id != *contract_id || deposit.deposit_circuit_id != *deposit_circuit_id
        {
            return Err(BlockchainError::DepositWithdrawPassedToWrongFunction);
        }
        executor_fees.push(deposit.fee);
        state_builder.batch_set(&zk::ZkDeltaPairs(
            [
                (
                    zk::ZkDataLocator(vec![i as u64, 0]),
                    Some(zk::ZkScalar::from(1)),
                ),
                (
                    zk::ZkDataLocator(vec![i as u64, 1]),
                    Some(deposit.amount.token_id.into()),
                ),
                (
                    zk::ZkDataLocator(vec![i as u64, 2]),
                    Some(zk::ZkScalar::from(deposit.amount.amount)),
                ),
                (zk::ZkDataLocator(vec![i as u64, 3]), Some(deposit.calldata)),
            ]
            .into(),
        ))?;

        chain.apply_deposit(deposit)?;
    }
    let aux_data = state_builder.compress()?;
    Ok((circuit.clone(), aux_data))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{RamKvStore, WriteOp};

    #[test]
    fn test_empty_deposit() {
        let chain = KvStoreChain::new(
            RamKvStore::new(),
            crate::config::blockchain::get_test_blockchain_config(),
        )
        .unwrap();

        let deposit_state_model = zk::ZkStateModel::List {
            item_type: Box::new(zk::ZkStateModel::Struct {
                field_types: vec![
                    zk::ZkStateModel::Scalar, // Enabled
                    zk::ZkStateModel::Scalar, // Token-id
                    zk::ZkStateModel::Scalar, // Amount
                    zk::ZkStateModel::Scalar, // Calldata
                ],
            }),
            log4_size: 1,
        };
        let empty_compressed =
            zk::ZkCompressedState::empty::<crate::core::ZkHasher>(deposit_state_model);

        let (ops, ((_, aux_data), exec_fees)) = chain
            .isolated(|chain| {
                let mut exec_fees = Vec::new();
                let contract_id = chain.config.mpn_config.mpn_contract_id;
                let contract = chain.get_contract(contract_id)?;
                Ok((
                    deposit(chain, &contract_id, &contract, &0, &[], &mut exec_fees)?,
                    exec_fees,
                ))
            })
            .unwrap();

        assert_eq!(aux_data, empty_compressed);
        assert!(ops.is_empty());
        assert!(exec_fees.is_empty());
    }

    #[test]
    fn test_single_deposit() {
        let chain = KvStoreChain::new(
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
            amount: Money::ziesha(123),
            fee: Money::ziesha(77),
            sig: None,
        };
        abc.sign_deposit(&mut cont_deposit);

        let (ops, ((_, aux_data), exec_fees)) = chain
            .isolated(|chain| {
                let mut exec_fees = Vec::new();
                let contract = chain.get_contract(contract_id)?;
                Ok((
                    deposit(
                        chain,
                        &contract_id,
                        &contract,
                        &0,
                        &[cont_deposit],
                        &mut exec_fees,
                    )?,
                    exec_fees,
                ))
            })
            .unwrap();
        let empty_leaf =
            <crate::core::ZkHasher as crate::zk::ZkHasher>::hash(&[zk::ZkScalar::from(0); 4]);
        let expected_hash = <crate::core::ZkHasher as crate::zk::ZkHasher>::hash(&[
            <crate::core::ZkHasher as crate::zk::ZkHasher>::hash(&[
                zk::ZkScalar::from(1),
                zk::ZkScalar::from(1),
                zk::ZkScalar::from(123),
                zk::ZkScalar::from(888),
            ]),
            empty_leaf,
            empty_leaf,
            empty_leaf,
        ]);

        let expected_aux = zk::ZkCompressedState {
            state_hash: expected_hash,
            state_size: 4,
        };

        assert_eq!(aux_data, expected_aux);

        let expected_ops = vec![
            WriteOp::Put(
                "ACB-ed8c19c6a4cf1460e961f7bae8eea54d437b9edac27cbeb09be32ae367adf9098a-Ziesha"
                    .into(),
                Amount(9800).into(),
            ),
            WriteOp::Put(
                "ACC-ed8c19c6a4cf1460e961f7bae8eea54d437b9edac27cbeb09be32ae367adf9098a".into(),
                Account { nonce: 1 }.into(),
            ),
            WriteOp::Put(
                "CAB-e97e9d34b101f5e98805dba10de8ad91fc61139d571cf4966fe36c7a56d6138b-Ziesha"
                    .into(),
                Amount(123).into(),
            ),
        ];

        assert_eq!(ops, expected_ops);
        assert_eq!(
            exec_fees,
            vec![Money {
                token_id: TokenId::Ziesha,
                amount: Amount(77)
            }]
        );
    }

    #[test]
    fn test_custom_token_deposit() {
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

        let abc = TxBuilder::new(&Vec::from("ABC"));
        let mut cont_deposit = ContractDeposit {
            memo: "".into(),
            src: abc.get_address(),
            contract_id,
            deposit_circuit_id: 0,
            calldata: zk::ZkScalar::from(888),
            nonce: 2,
            amount: Money {
                token_id: kiwi_token_id,
                amount: Amount(10000),
            },
            fee: Money::ziesha(321),
            sig: None,
        };
        abc.sign_deposit(&mut cont_deposit);

        let (ops, ((_, aux_data), exec_fees)) = chain
            .isolated(|chain| {
                let mut exec_fees = Vec::new();
                let contract = chain.get_contract(contract_id)?;
                Ok((
                    deposit(
                        chain,
                        &contract_id,
                        &contract,
                        &0,
                        &[cont_deposit],
                        &mut exec_fees,
                    )?,
                    exec_fees,
                ))
            })
            .unwrap();
        let empty_leaf =
            <crate::core::ZkHasher as crate::zk::ZkHasher>::hash(&[zk::ZkScalar::from(0); 4]);
        let expected_hash = <crate::core::ZkHasher as crate::zk::ZkHasher>::hash(&[
            <crate::core::ZkHasher as crate::zk::ZkHasher>::hash(&[
                zk::ZkScalar::from(1),
                zk::ZkScalar::from(kiwi_token_id),
                zk::ZkScalar::from(10000),
                zk::ZkScalar::from(888),
            ]),
            empty_leaf,
            empty_leaf,
            empty_leaf,
        ]);

        let expected_aux = zk::ZkCompressedState {
            state_hash: expected_hash,
            state_size: 4,
        };

        assert_eq!(aux_data, expected_aux);

        let expected_ops = vec![
            WriteOp::Put(
                "ACB-ed8c19c6a4cf1460e961f7bae8eea54d437b9edac27cbeb09be32ae367adf9098a-0x113c5707d6ab03dd78c9f8455f3b4096c8c5e9312076aafe38d79c4d8eb2d096"
                    .into(),
                Amount(90000).into(),
            ),
            WriteOp::Put(
                "ACB-ed8c19c6a4cf1460e961f7bae8eea54d437b9edac27cbeb09be32ae367adf9098a-Ziesha"
                    .into(),
                Amount(9679).into(),
            ),
            WriteOp::Put(
                "ACC-ed8c19c6a4cf1460e961f7bae8eea54d437b9edac27cbeb09be32ae367adf9098a".into(),
                Account { nonce: 2 }.into(),
            ),
            WriteOp::Put(
                "CAB-e97e9d34b101f5e98805dba10de8ad91fc61139d571cf4966fe36c7a56d6138b-0x113c5707d6ab03dd78c9f8455f3b4096c8c5e9312076aafe38d79c4d8eb2d096"
                    .into(),
                Amount(10000).into(),
            ),
        ];

        assert_eq!(ops, expected_ops);
        assert_eq!(
            exec_fees,
            vec![Money {
                token_id: TokenId::Ziesha,
                amount: Amount(321)
            }]
        );
    }
}
