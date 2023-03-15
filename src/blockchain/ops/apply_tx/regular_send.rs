use super::*;

pub fn regular_send<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    tx_src: Address,
    entries: &[RegularSendEntry],
) -> Result<(), BlockchainError> {
    for entry in entries {
        if entry.dst != tx_src {
            let mut src_bal = chain.get_balance(tx_src.clone(), entry.amount.token_id)?;

            if src_bal < entry.amount.amount {
                return Err(BlockchainError::BalanceInsufficient);
            }
            src_bal -= entry.amount.amount;
            chain.database.update(&[WriteOp::Put(
                keys::account_balance(&tx_src, entry.amount.token_id),
                src_bal.into(),
            )])?;

            let mut dst_bal = chain.get_balance(entry.dst.clone(), entry.amount.token_id)?;
            dst_bal += entry.amount.amount;

            chain.database.update(&[WriteOp::Put(
                keys::account_balance(&entry.dst, entry.amount.token_id),
                dst_bal.into(),
            )])?;
        }
    }
    Ok(())
}
