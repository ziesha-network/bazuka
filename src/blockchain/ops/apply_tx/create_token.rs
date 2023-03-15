use super::*;

pub fn create_token<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    tx_src: Address,
    token_id: TokenId,
    token: &Token,
) -> Result<(), BlockchainError> {
    if chain.get_token(token_id)?.is_some() {
        return Err(BlockchainError::TokenAlreadyExists);
    } else {
        if !token.validate() {
            return Err(BlockchainError::TokenBadNameSymbol);
        }
        chain.database.update(&[WriteOp::Put(
            keys::account_balance(&tx_src, token_id),
            token.supply.into(),
        )])?;
        chain
            .database
            .update(&[WriteOp::Put(keys::token(&token_id), token.into())])?;
    }
    Ok(())
}
