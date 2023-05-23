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
            if chain.config.check_validator {
                if let Some(proof) = block.header.proof_of_stake.proof.clone() {
                    if !chain.is_validator(
                        block.header.proof_of_stake.timestamp,
                        block.header.proof_of_stake.validator.clone(),
                        proof,
                    )? {
                        return Err(BlockchainError::UnelectedValidator);
                    }
                } else {
                    return Err(BlockchainError::UnelectedValidator); // TODO: Separate error
                }
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
                ..
            } = &tx.data
            {
                if contract_id.clone() == chain.config.mpn_config.mpn_contract_id {
                    for update in updates.iter() {
                        match update.data {
                            ContractUpdateData::Deposit { .. } => {
                                num_mpn_contract_deposits += 1;
                            }
                            ContractUpdateData::Withdraw { .. } => {
                                num_mpn_contract_withdraws += 1;
                            }
                            ContractUpdateData::FunctionCall { .. } => {
                                num_mpn_function_calls += 1;
                            }
                        }
                    }
                }
            }

            body_size += tx.size();
            chain.apply_tx(tx, is_genesis)?;
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

        if curr_height > 0 {
            let mut new_randomness = chain.epoch_randomness()?;
            let tip_epoch = chain
                .epoch_slot(chain.get_tip()?.proof_of_stake.timestamp)
                .0;
            let block_epoch = chain.epoch_slot(block.header.proof_of_stake.timestamp).0;
            if block_epoch > tip_epoch {
                let mut head = chain.get_tip()?;
                while chain.epoch_slot(head.proof_of_stake.timestamp).0 == tip_epoch
                    && head.number > 0
                {
                    // TODO: WAS A TESTNET SECURITY BUG, FIX LATER!
                    let output_hash = if head.number < 700 {
                        head.proof_of_stake
                            .proof
                            .map(|p| Hasher::hash(&Into::<Vec<u8>>::into(p.vrf_output.clone())))
                            .unwrap_or_default()
                    } else {
                        head.hash()
                    };
                    let mut preimage: Vec<u8> = new_randomness.to_vec();
                    preimage.extend(output_hash);
                    new_randomness = Hasher::hash(&preimage);
                    head = chain.get_header(head.number - 1)?;
                }
                chain
                    .database
                    .update(&[WriteOp::Put(keys::randomness(), new_randomness.into())])?;
            }
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
        ])?;

        let rollback = chain.database.rollback()?;

        chain.database.update(&[WriteOp::Put(
            keys::rollback(block.header.number),
            rollback.into(),
        )])?;

        Ok(())
    })?;

    chain.database.update(&ops)?;
    Ok(())
}
