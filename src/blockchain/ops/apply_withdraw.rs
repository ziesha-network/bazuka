use super::*;

pub fn apply_withdraw<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    withdraw: &ContractWithdraw,
) -> Result<(), BlockchainError> {
    let (ops, _) = chain.isolated(|chain| {
        if withdraw.amount.token_id == withdraw.fee.token_id {
            let mut contract_balance =
                chain.get_contract_balance(withdraw.contract_id, withdraw.amount.token_id)?;

            if contract_balance < withdraw.amount.amount + withdraw.fee.amount {
                return Err(BlockchainError::ContractBalanceInsufficient);
            }
            contract_balance -= withdraw.amount.amount + withdraw.fee.amount;
            chain.database.update(&[WriteOp::Put(
                keys::contract_balance(&withdraw.contract_id, withdraw.amount.token_id),
                contract_balance.into(),
            )])?;
        } else {
            let mut contract_balance =
                chain.get_contract_balance(withdraw.contract_id, withdraw.amount.token_id)?;
            let mut contract_fee_balance =
                chain.get_contract_balance(withdraw.contract_id, withdraw.fee.token_id)?;

            if contract_balance < withdraw.amount.amount {
                return Err(BlockchainError::ContractBalanceInsufficient);
            }
            if contract_fee_balance < withdraw.fee.amount {
                return Err(BlockchainError::ContractBalanceInsufficient);
            }
            contract_fee_balance -= withdraw.fee.amount;
            contract_balance -= withdraw.amount.amount;
            chain.database.update(&[WriteOp::Put(
                keys::contract_balance(&withdraw.contract_id, withdraw.amount.token_id),
                contract_balance.into(),
            )])?;
            chain.database.update(&[WriteOp::Put(
                keys::contract_balance(&withdraw.contract_id, withdraw.fee.token_id),
                contract_fee_balance.into(),
            )])?;
        }

        let mut addr_balance = chain.get_balance(withdraw.dst.clone(), withdraw.amount.token_id)?;
        addr_balance += withdraw.amount.amount;
        chain.database.update(&[WriteOp::Put(
            keys::account_balance(&withdraw.dst, withdraw.amount.token_id),
            addr_balance.into(),
        )])?;

        Ok(())
    })?;
    chain.database.update(&ops)?;
    Ok(())
}
