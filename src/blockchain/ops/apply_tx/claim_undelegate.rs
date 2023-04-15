use super::*;

pub fn claim_undelegate<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    tx_src: Address,
    undelegation_id: UndelegationId,
) -> Result<(), BlockchainError> {
    if let Some(undelegation) = chain.get_undelegation(tx_src.clone(), undelegation_id.clone())? {
        if chain.get_height()? >= undelegation.unlocks_on {
            let mut src_bal = chain.get_balance(tx_src.clone(), TokenId::Ziesha)?;
            src_bal += undelegation.amount;
            chain.database.update(&[
                WriteOp::Remove(
                    keys::UndelegationDbKey {
                        undelegator: tx_src.clone(),
                        undelegation_id: undelegation_id,
                    }
                    .into(),
                ),
                WriteOp::Put(
                    keys::account_balance(&tx_src, TokenId::Ziesha),
                    src_bal.into(),
                ),
            ])?;
        } else {
            return Err(BlockchainError::UndelegationLocked);
        }
    } else {
        return Err(BlockchainError::UndelegationNotFound);
    }
    Ok(())
}
