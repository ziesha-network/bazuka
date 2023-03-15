use super::*;

pub fn apply_tx<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    tx: &Transaction,
    allow_treasury: bool,
) -> Result<TxSideEffect, BlockchainError> {
    let (ops, side_effect) = chain.isolated(|chain| {
        let mut side_effect = TxSideEffect::Nothing;

        if tx.src == None && !allow_treasury {
            return Err(BlockchainError::IllegalTreasuryAccess);
        }

        if tx.fee.token_id != TokenId::Ziesha {
            return Err(BlockchainError::OnlyZieshaFeesAccepted);
        }

        if tx.memo.len() > chain.config.max_memo_length {
            return Err(BlockchainError::MemoTooLong);
        }

        let tx_src = tx.src.clone().unwrap_or_default(); // Default is treasury account!

        let mut acc_src = chain.get_account(tx_src.clone())?;
        let mut acc_bal = chain.get_balance(tx_src.clone(), tx.fee.token_id)?;

        if tx.nonce != acc_src.nonce + 1 {
            return Err(BlockchainError::InvalidTransactionNonce);
        }

        if acc_bal < tx.fee.amount {
            return Err(BlockchainError::BalanceInsufficient);
        }

        acc_bal -= tx.fee.amount;
        acc_src.nonce += 1;

        chain
            .database
            .update(&[WriteOp::Put(keys::account(&tx_src), acc_src.into())])?;
        chain.database.update(&[WriteOp::Put(
            keys::account_balance(&tx_src, tx.fee.token_id),
            acc_bal.into(),
        )])?;

        match &tx.data {
            TransactionData::UpdateStaker { vrf_pub_key } => {
                chain.database.update(&[WriteOp::Put(
                    keys::staker(&tx_src),
                    Staker {
                        vrf_pub_key: vrf_pub_key.clone(),
                    }
                    .into(),
                )])?;
            }
            TransactionData::Delegate {
                amount,
                to,
                reverse,
            } => {
                let mut src_bal = chain.get_balance(tx_src.clone(), TokenId::Ziesha)?;
                if !reverse {
                    if src_bal < *amount {
                        return Err(BlockchainError::BalanceInsufficient);
                    }
                    src_bal -= *amount;
                } else {
                    src_bal += *amount;
                }
                chain.database.update(&[WriteOp::Put(
                    keys::account_balance(&tx_src, TokenId::Ziesha),
                    src_bal.into(),
                )])?;

                let mut delegate = chain.get_delegate(tx_src.clone(), to.clone())?;
                let old_delegate = delegate.amount;
                if !reverse {
                    delegate.amount += *amount;
                } else {
                    if delegate.amount < *amount {
                        return Err(BlockchainError::BalanceInsufficient);
                    }
                    delegate.amount -= *amount;
                }
                chain.database.update(&[WriteOp::Put(
                    keys::delegate(&tx_src, to),
                    delegate.clone().into(),
                )])?;

                let old_stake = chain.get_stake(to.clone())?;
                let new_stake = old_stake + *amount;
                chain.database.update(&[
                    WriteOp::Remove(keys::delegatee_rank(&tx_src, old_delegate, &to)),
                    WriteOp::Put(
                        keys::delegatee_rank(&tx_src, delegate.amount, &to),
                        ().into(),
                    ),
                    WriteOp::Remove(keys::delegator_rank(&to, old_delegate, &tx_src)),
                    WriteOp::Put(
                        keys::delegator_rank(&to, delegate.amount, &tx_src),
                        ().into(),
                    ),
                    WriteOp::Remove(keys::staker_rank(old_stake, &to)),
                    WriteOp::Put(keys::staker_rank(new_stake, &to), ().into()),
                    WriteOp::Put(keys::stake(&to), new_stake.into()),
                ])?;
            }
            TransactionData::CreateToken { token } => {
                let token_id = {
                    let tid = TokenId::new(tx);
                    if tid == chain.config.ziesha_token_id {
                        TokenId::Ziesha
                    } else {
                        tid
                    }
                };
                if chain.get_token(token_id)?.is_some() {
                    return Err(BlockchainError::TokenAlreadyExists);
                } else {
                    if !token.validate() {
                        return Err(BlockchainError::TokenBadNameSymbol);
                    }
                    chain.database.update(&[WriteOp::Put(
                        keys::account_balance(&tx_src, token_id),
                        token.supply.into(),
                    )])?;
                    chain
                        .database
                        .update(&[WriteOp::Put(keys::token(&token_id), token.into())])?;
                }
            }
            TransactionData::UpdateToken { token_id, update } => {
                if let Some(mut token) = chain.get_token(*token_id)? {
                    if let Some(minter) = token.minter.clone() {
                        if minter == tx_src {
                            match update {
                                TokenUpdate::Mint { amount } => {
                                    let mut bal = chain.get_balance(tx_src.clone(), *token_id)?;
                                    if bal + *amount < bal || token.supply + *amount < token.supply
                                    {
                                        return Err(BlockchainError::TokenSupplyOverflow);
                                    }
                                    bal += *amount;
                                    token.supply += *amount;
                                    chain.database.update(&[WriteOp::Put(
                                        keys::token(token_id),
                                        (&token).into(),
                                    )])?;
                                    chain.database.update(&[WriteOp::Put(
                                        keys::account_balance(&tx_src, *token_id),
                                        bal.into(),
                                    )])?;
                                }
                                TokenUpdate::ChangeMinter { minter } => {
                                    token.minter = Some(minter.clone());
                                    chain.database.update(&[WriteOp::Put(
                                        keys::token(token_id),
                                        (&token).into(),
                                    )])?;
                                }
                            }
                        } else {
                            return Err(BlockchainError::TokenUpdatePermissionDenied);
                        }
                    } else {
                        return Err(BlockchainError::TokenNotUpdatable);
                    }
                } else {
                    return Err(BlockchainError::TokenNotFound);
                }
            }
            TransactionData::RegularSend { entries } => {
                for entry in entries {
                    if entry.dst != tx_src {
                        let mut src_bal =
                            chain.get_balance(tx_src.clone(), entry.amount.token_id)?;

                        if src_bal < entry.amount.amount {
                            return Err(BlockchainError::BalanceInsufficient);
                        }
                        src_bal -= entry.amount.amount;
                        chain.database.update(&[WriteOp::Put(
                            keys::account_balance(&tx_src, entry.amount.token_id),
                            src_bal.into(),
                        )])?;

                        let mut dst_bal =
                            chain.get_balance(entry.dst.clone(), entry.amount.token_id)?;
                        dst_bal += entry.amount.amount;

                        chain.database.update(&[WriteOp::Put(
                            keys::account_balance(&entry.dst, entry.amount.token_id),
                            dst_bal.into(),
                        )])?;
                    }
                }
            }
            TransactionData::CreateContract { contract } => {
                if !contract.state_model.is_valid::<CoreZkHasher>() {
                    return Err(BlockchainError::InvalidStateModel);
                }
                let contract_id = ContractId::new(tx);
                chain.database.update(&[WriteOp::Put(
                    keys::contract(&contract_id),
                    contract.clone().into(),
                )])?;
                let compressed_empty =
                    zk::ZkCompressedState::empty::<CoreZkHasher>(contract.state_model.clone());
                chain.database.update(&[WriteOp::Put(
                    keys::contract_account(&contract_id),
                    ContractAccount {
                        compressed_state: contract.initial_state,
                        height: 1,
                    }
                    .into(),
                )])?;
                chain.database.update(&[WriteOp::Put(
                    keys::compressed_state_at(&contract_id, 1),
                    contract.initial_state.into(),
                )])?;
                side_effect = TxSideEffect::StateChange {
                    contract_id,
                    state_change: ZkCompressedStateChange {
                        prev_height: 0,
                        prev_state: compressed_empty,
                        state: contract.initial_state,
                    },
                };
            }
            TransactionData::UpdateContract {
                contract_id,
                updates,
            } => {
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
                            let mut state_builder =
                                zk::ZkStateBuilder::<CoreZkHasher>::new(state_model);
                            for (i, deposit) in deposits.iter().enumerate() {
                                if deposit.contract_id != *contract_id
                                    || deposit.deposit_circuit_id != *deposit_circuit_id
                                {
                                    return Err(
                                        BlockchainError::DepositWithdrawPassedToWrongFunction,
                                    );
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
                                        (
                                            zk::ZkDataLocator(vec![i as u64, 3]),
                                            Some(deposit.calldata),
                                        ),
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
                            let mut state_builder =
                                zk::ZkStateBuilder::<CoreZkHasher>::new(state_model);
                            for (i, withdraw) in withdraws.iter().enumerate() {
                                if withdraw.contract_id != *contract_id
                                    || withdraw.withdraw_circuit_id != *withdraw_circuit_id
                                {
                                    return Err(
                                        BlockchainError::DepositWithdrawPassedToWrongFunction,
                                    );
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

                            let mut cont_balance =
                                chain.get_contract_balance(*contract_id, fee.token_id)?;
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
                side_effect = TxSideEffect::StateChange {
                    contract_id: *contract_id,
                    state_change: ZkCompressedStateChange {
                        prev_height: prev_account.height,
                        prev_state: prev_account.compressed_state,
                        state: cont_account.compressed_state,
                    },
                };
            }
        }

        // Fees go to the Treasury account first
        if tx.src != None {
            let mut treasury_balance = chain.get_balance(Default::default(), tx.fee.token_id)?;
            treasury_balance += tx.fee.amount;
            chain.database.update(&[WriteOp::Put(
                keys::account_balance(&Default::default(), tx.fee.token_id),
                treasury_balance.into(),
            )])?;
        }

        Ok(side_effect)
    })?;

    chain.database.update(&ops)?;
    Ok(side_effect)
}
