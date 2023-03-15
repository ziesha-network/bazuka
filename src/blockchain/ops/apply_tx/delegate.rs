use super::*;

pub fn delegate<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    tx_src: Address,
    amount: Amount,
    to: Address,
    reverse: bool,
) -> Result<(), BlockchainError> {
    let mut src_bal = chain.get_balance(tx_src.clone(), TokenId::Ziesha)?;
    if !reverse {
        if src_bal < amount {
            return Err(BlockchainError::BalanceInsufficient);
        }
        src_bal -= amount;
    } else {
        src_bal += amount;
    }
    chain.database.update(&[WriteOp::Put(
        keys::account_balance(&tx_src, TokenId::Ziesha),
        src_bal.into(),
    )])?;

    let mut delegate = chain.get_delegate(tx_src.clone(), to.clone())?;
    let old_delegate = delegate.amount;
    if !reverse {
        delegate.amount += amount;
    } else {
        if delegate.amount < amount {
            return Err(BlockchainError::BalanceInsufficient);
        }
        delegate.amount -= amount;
    }
    chain.database.update(&[WriteOp::Put(
        keys::delegate(&tx_src, &to),
        delegate.clone().into(),
    )])?;

    let old_stake = chain.get_stake(to.clone())?;
    let new_stake = old_stake + amount;
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
    Ok(())
}
