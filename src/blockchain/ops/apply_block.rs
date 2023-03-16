use super::*;

pub fn apply_block<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    block: &Block,
) -> Result<(), BlockchainError> {
    let (ops, _) = chain.isolated(|chain| {
        let curr_height = chain.get_height()?;

        if let Some(height_limit) = chain.config.testnet_height_limit {
            if block.header.number >= height_limit {
                return Err(BlockchainError::TestnetHeightLimitReached);
            }
        }

        let is_genesis = block.header.number == 0;

        if curr_height > 0 {
            if block.merkle_tree().root() != block.header.block_root {
                return Err(BlockchainError::InvalidMerkleRoot);
            }

            chain.will_extend(curr_height, &[block.header.clone()])?;
        }

        if !is_genesis {
            if chain.config.check_validator
                && !chain.is_validator(
                    block.header.proof_of_stake.timestamp,
                    block.header.proof_of_stake.validator.clone(),
                    block.header.proof_of_stake.proof.clone(),
                )?
            {
                return Err(BlockchainError::UnelectedValidator);
            }
            // WARN: Sum will be invalid if fees are not in Ziesha
            let fee_sum = Amount(
                block
                    .body
                    .iter()
                    .map(|t| -> u64 { t.fee.amount.into() })
                    .sum(),
            );
            chain.pay_validator_and_delegators(
                block.header.proof_of_stake.validator.clone(),
                fee_sum,
            )?;
        }

        let mut body_size = 0usize;
        let mut state_size_delta = 0isize;
        let mut state_updates: HashMap<ContractId, ZkCompressedStateChange> = HashMap::new();
        let mut outdated_contracts = chain.get_outdated_contracts()?;

        if !is_genesis && !block.body.par_iter().all(|tx| tx.verify_signature()) {
            return Err(BlockchainError::SignatureError);
        }

        let mut num_mpn_function_calls = 0;
        let mut num_mpn_contract_deposits = 0;
        let mut num_mpn_contract_withdraws = 0;

        for tx in block.body.iter() {
            // Count MPN updates
            if let TransactionData::UpdateContract {
                contract_id,
                updates,
            } = &tx.data
            {
                if contract_id.clone() == chain.config.mpn_config.mpn_contract_id {
                    for update in updates.iter() {
                        match update {
                            ContractUpdate::Deposit { .. } => {
                                num_mpn_contract_deposits += 1;
                            }
                            ContractUpdate::Withdraw { .. } => {
                                num_mpn_contract_withdraws += 1;
                            }
                            ContractUpdate::FunctionCall { .. } => {
                                num_mpn_function_calls += 1;
                            }
                        }
                    }
                }
            }

            body_size += tx.size();
            // All genesis block txs are allowed to get from Treasury
            if let TxSideEffect::StateChange {
                contract_id,
                state_change,
            } = chain.apply_tx(tx, is_genesis)?
            {
                state_size_delta +=
                    state_change.state.size() as isize - state_change.prev_state.size() as isize;
                if !state_updates.contains_key(&contract_id) {
                    state_updates.insert(contract_id, state_change.clone());
                } else {
                    return Err(BlockchainError::SingleUpdateAllowedPerContract);
                }
                if !outdated_contracts.contains(&contract_id) {
                    outdated_contracts.push(contract_id);
                }
            }
        }

        if !is_genesis
            && (num_mpn_function_calls < chain.config.mpn_config.mpn_num_update_batches
                || num_mpn_contract_deposits < chain.config.mpn_config.mpn_num_deposit_batches
                || num_mpn_contract_withdraws < chain.config.mpn_config.mpn_num_withdraw_batches)
        {
            return Err(BlockchainError::InsufficientMpnUpdates);
        }

        if body_size > chain.config.max_block_size {
            return Err(BlockchainError::BlockTooBig);
        }

        if state_size_delta > chain.config.max_delta_count as isize {
            return Err(BlockchainError::StateDeltaTooBig);
        }

        chain.database.update(&[
            WriteOp::Put(keys::height(), (curr_height + 1).into()),
            WriteOp::Put(
                keys::header(block.header.number),
                block.header.clone().into(),
            ),
            WriteOp::Put(keys::block(block.header.number), block.into()),
            WriteOp::Put(
                keys::merkle(block.header.number),
                block.merkle_tree().into(),
            ),
            WriteOp::Put(keys::contract_updates(), state_updates.into()),
        ])?;

        let rollback = chain.database.rollback()?;

        chain.database.update(&[
            WriteOp::Put(keys::rollback(block.header.number), rollback.into()),
            if outdated_contracts.is_empty() {
                WriteOp::Remove(keys::outdated())
            } else {
                WriteOp::Put(keys::outdated(), outdated_contracts.clone().into())
            },
        ])?;

        Ok(())
    })?;

    chain.database.update(&ops)?;
    Ok(())
}
