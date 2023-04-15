use super::*;

pub fn init_undelegate<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    undelegation_id: UndelegationId,
    tx_src: Address,
    amount: Amount,
    from: Address,
) -> Result<(), BlockchainError> {
    let undelegation = Undelegation {
        amount,
        unlocks_on: chain.get_height()? + 10, // TODO: Put undelegation period into config
    };
    let mut delegate = chain.get_delegate(tx_src.clone(), from.clone())?;
    let old_delegate = delegate.amount;
    if delegate.amount < amount {
        return Err(BlockchainError::BalanceInsufficient);
    }
    delegate.amount -= amount;
    chain.database.update(&[WriteOp::Put(
        keys::delegate(&tx_src, &from),
        delegate.clone().into(),
    )])?;

    let old_stake = chain.get_stake(from.clone())?;
    if old_stake < amount {
        return Err(BlockchainError::Inconsistency);
    }
    let new_stake = old_stake - amount;
    chain.database.update(&[
        WriteOp::Put(
            keys::UndelegationDbKey {
                undelegator: tx_src.clone(),
                undelegation_id: undelegation_id,
            }
            .into(),
            undelegation.into(),
        ),
        WriteOp::Remove(
            keys::DelegateeRankDbKey {
                delegator: tx_src.clone(),
                delegatee: from.clone(),
                amount: old_delegate,
            }
            .into(),
        ),
        WriteOp::Put(
            keys::DelegateeRankDbKey {
                delegator: tx_src.clone(),
                delegatee: from.clone(),
                amount: delegate.amount,
            }
            .into(),
            ().into(),
        ),
        WriteOp::Remove(
            keys::DelegatorRankDbKey {
                delegator: tx_src.clone(),
                delegatee: from.clone(),
                amount: old_delegate,
            }
            .into(),
        ),
        WriteOp::Put(
            keys::DelegatorRankDbKey {
                delegator: tx_src.clone(),
                delegatee: from.clone(),
                amount: delegate.amount,
            }
            .into(),
            ().into(),
        ),
        WriteOp::Remove(
            keys::StakerRankDbKey {
                address: from.clone(),
                amount: old_stake,
            }
            .into(),
        ),
        WriteOp::Put(
            keys::StakerRankDbKey {
                address: from.clone(),
                amount: new_stake,
            }
            .into(),
            ().into(),
        ),
        WriteOp::Put(keys::stake(&from), new_stake.into()),
    ])?;
    Ok(())
}
