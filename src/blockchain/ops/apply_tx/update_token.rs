use super::*;

pub fn update_token<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    tx_src: Address,
    token_id: &TokenId,
    update: &TokenUpdate,
) -> Result<(), BlockchainError> {
    if let Some(mut token) = chain.get_token(*token_id)? {
        if let Some(minter) = token.minter.clone() {
            if minter == tx_src {
                match update {
                    TokenUpdate::Mint { amount } => {
                        let mut bal = chain.get_balance(tx_src.clone(), *token_id)?;
                        if bal + *amount < bal || token.supply + *amount < token.supply {
                            return Err(BlockchainError::TokenSupplyOverflow);
                        }
                        bal += *amount;
                        token.supply += *amount;
                        chain
                            .database
                            .update(&[WriteOp::Put(keys::token(token_id), (&token).into())])?;
                        chain.database.update(&[WriteOp::Put(
                            keys::account_balance(&tx_src, *token_id),
                            bal.into(),
                        )])?;
                    }
                    TokenUpdate::ChangeMinter { minter } => {
                        token.minter = Some(minter.clone());
                        chain
                            .database
                            .update(&[WriteOp::Put(keys::token(token_id), (&token).into())])?;
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
    Ok(())
}
