use super::*;

pub fn select_transactions<K: KvStore>(
    chain: &KvStoreChain<K>,
    validator: Address,
    txs: &[TransactionAndDelta],
    check: bool,
) -> Result<Vec<TransactionAndDelta>, BlockchainError> {
    let mut sorted = txs
        .iter()
        .filter(|t| t.tx.fee.token_id == TokenId::Ziesha)
        .cloned()
        .collect::<Vec<_>>();
    // WARN: Sort will be invalid if not all fees are specified in Ziesha
    sorted.sort_unstable_by_key(|tx| {
        let cost = tx.tx.size();
        let is_mpn = if let TransactionData::UpdateContract { contract_id, .. } = &tx.tx.data {
            *contract_id == chain.config.mpn_config.mpn_contract_id
        } else {
            false
        };
        (
            is_mpn,
            Into::<u64>::into(tx.tx.fee.amount) / cost as u64,
            -(tx.tx.nonce as i32),
        )
    });
    if !check {
        return Ok(sorted);
    }
    let (_, result) = chain.isolated(|chain| {
        // Safe to consider a 0 fee-sum
        chain.pay_validator_and_delegators(validator, Amount(0))?;

        let mut result = Vec::new();
        let mut block_sz = 0usize;
        let mut delta_cnt = 0isize;
        for tx in sorted.into_iter().rev() {
            match chain.isolated(|chain| chain.apply_tx(&tx.tx, false)) {
                Ok((ops, eff)) => {
                    let delta_diff = if let TxSideEffect::StateChange { state_change, .. } = eff {
                        state_change.state.size() as isize - state_change.prev_state.size() as isize
                    } else {
                        0
                    };
                    let block_diff = tx.tx.size();
                    if delta_cnt + delta_diff <= chain.config.max_delta_count as isize
                        && block_sz + block_diff <= chain.config.max_block_size
                        && tx.tx.verify_signature()
                    {
                        delta_cnt += delta_diff;
                        block_sz += block_diff;
                        chain.database.update(&ops)?;
                        result.push(tx);
                    }
                }
                Err(e) => {
                    let is_mpn =
                        if let TransactionData::UpdateContract { contract_id, .. } = &tx.tx.data {
                            *contract_id == chain.config.mpn_config.mpn_contract_id
                        } else {
                            false
                        };
                    if is_mpn {
                        log::error!("MPN transaction rejected: {}", e);
                    }
                }
            }
        }
        Ok(result)
    })?;
    Ok(result)
}
