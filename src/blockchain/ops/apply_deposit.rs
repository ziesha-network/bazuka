use super::*;

pub fn apply_deposit<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    deposit: &ContractDeposit,
) -> Result<(), BlockchainError> {
    let (ops, _) = chain.isolated(|chain| {
        if !deposit.verify_signature() {
            return Err(BlockchainError::InvalidContractPaymentSignature);
        }

        let mut deposit_nonce =
            chain.get_deposit_nonce(deposit.src.clone(), deposit.contract_id.clone())?;
        if deposit.nonce != deposit_nonce + 1 {
            return Err(BlockchainError::InvalidTransactionNonce);
        }
        deposit_nonce += 1;
        chain.database.update(&[WriteOp::Put(
            keys::deposit_nonce(&deposit.src, &deposit.contract_id),
            deposit_nonce.into(),
        )])?;

        if deposit.amount.token_id == deposit.fee.token_id {
            let mut addr_balance =
                chain.get_balance(deposit.src.clone(), deposit.amount.token_id)?;

            if addr_balance < deposit.amount.amount + deposit.fee.amount {
                return Err(BlockchainError::BalanceInsufficient);
            }
            addr_balance -= deposit.amount.amount + deposit.fee.amount;
            chain.database.update(&[WriteOp::Put(
                keys::account_balance(&deposit.src, deposit.amount.token_id),
                addr_balance.into(),
            )])?;
        } else {
            let mut addr_balance =
                chain.get_balance(deposit.src.clone(), deposit.amount.token_id)?;
            let mut addr_fee_balance =
                chain.get_balance(deposit.src.clone(), deposit.fee.token_id)?;

            if addr_balance < deposit.amount.amount {
                return Err(BlockchainError::BalanceInsufficient);
            }
            if addr_fee_balance < deposit.fee.amount {
                return Err(BlockchainError::BalanceInsufficient);
            }
            addr_fee_balance -= deposit.fee.amount;
            addr_balance -= deposit.amount.amount;
            chain.database.update(&[WriteOp::Put(
                keys::account_balance(&deposit.src, deposit.amount.token_id),
                addr_balance.into(),
            )])?;
            chain.database.update(&[WriteOp::Put(
                keys::account_balance(&deposit.src, deposit.fee.token_id),
                addr_fee_balance.into(),
            )])?;
        }

        let mut contract_balance =
            chain.get_contract_balance(deposit.contract_id, deposit.amount.token_id)?;
        contract_balance += deposit.amount.amount;
        chain.database.update(&[WriteOp::Put(
            keys::contract_balance(&deposit.contract_id, deposit.amount.token_id),
            contract_balance.into(),
        )])?;
        Ok(())
    })?;
    chain.database.update(&ops)?;
    Ok(())
}
