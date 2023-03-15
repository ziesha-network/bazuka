use super::*;

pub fn update_contract<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    tx_src: Address,
    contract_id: &ContractId,
    updates: &[ContractUpdate],
) -> Result<TxSideEffect, BlockchainError> {
    let contract = chain.get_contract(*contract_id)?;
    let mut executor_fees = Vec::new();

    let prev_account = chain.get_contract_account(*contract_id)?;

    // Increase height
    let mut cont_account = chain.get_contract_account(*contract_id)?;
    cont_account.height += 1;
    chain.database.update(&[WriteOp::Put(
        keys::contract_account(contract_id),
        cont_account.into(),
    )])?;

    for update in updates {
        let (circuit, aux_data, next_state, proof) = match update {
            ContractUpdate::Deposit {
                deposit_circuit_id,
                deposits,
                next_state,
                proof,
            } => {
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
                    if deposit.contract_id != *contract_id
                        || deposit.deposit_circuit_id != *deposit_circuit_id
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
                (circuit, aux_data, next_state, proof)
            }
            ContractUpdate::Withdraw {
                withdraw_circuit_id,
                withdraws,
                next_state,
                proof,
            } => {
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
                (circuit, aux_data, next_state, proof)
            }
            ContractUpdate::FunctionCall {
                function_id,
                next_state,
                proof,
                fee,
            } => {
                executor_fees.push(*fee);

                let mut cont_balance = chain.get_contract_balance(*contract_id, fee.token_id)?;
                if cont_balance < fee.amount {
                    return Err(BlockchainError::BalanceInsufficient);
                }
                cont_balance -= fee.amount;
                chain.database.update(&[WriteOp::Put(
                    keys::contract_balance(contract_id, fee.token_id),
                    cont_balance.into(),
                )])?;

                let func = contract
                    .functions
                    .get(*function_id as usize)
                    .ok_or(BlockchainError::ContractFunctionNotFound)?;
                let circuit = &func.verifier_key;
                let mut state_builder =
                    zk::ZkStateBuilder::<CoreZkHasher>::new(zk::ZkStateModel::Struct {
                        field_types: vec![
                            zk::ZkStateModel::Scalar, // Token-id
                            zk::ZkStateModel::Scalar, // Total fee
                        ],
                    });
                state_builder.batch_set(&zk::ZkDeltaPairs(
                    [
                        (zk::ZkDataLocator(vec![0]), Some(fee.token_id.into())),
                        (
                            zk::ZkDataLocator(vec![1]),
                            Some(zk::ZkScalar::from(fee.amount)),
                        ),
                    ]
                    .into(),
                ))?;
                let aux_data = state_builder.compress()?;
                (circuit, aux_data, next_state, proof)
            }
        };

        let mut cont_account = chain.get_contract_account(*contract_id)?;
        if !zk::check_proof(
            circuit,
            prev_account.height,
            &cont_account.compressed_state,
            &aux_data,
            next_state,
            proof,
        ) {
            return Err(BlockchainError::IncorrectZkProof);
        }
        cont_account.compressed_state = *next_state;
        chain.database.update(&[WriteOp::Put(
            keys::contract_account(contract_id),
            cont_account.into(),
        )])?;
    }

    for fee in executor_fees {
        let mut acc_bal = chain.get_balance(tx_src.clone(), fee.token_id)?;
        acc_bal += fee.amount;
        chain.database.update(&[WriteOp::Put(
            keys::account_balance(&tx_src, fee.token_id),
            acc_bal.into(),
        )])?;
    }

    let cont_account = chain.get_contract_account(*contract_id)?;

    chain.database.update(&[WriteOp::Put(
        keys::compressed_state_at(contract_id, cont_account.height),
        cont_account.compressed_state.into(),
    )])?;
    Ok(TxSideEffect::StateChange {
        contract_id: *contract_id,
        state_change: ZkCompressedStateChange {
            prev_height: prev_account.height,
            prev_state: prev_account.compressed_state,
            state: cont_account.compressed_state,
        },
    })
}
