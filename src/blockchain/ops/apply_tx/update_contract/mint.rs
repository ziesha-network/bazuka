use super::*;

pub fn mint<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    contract_id: &ContractId,
    contract: &zk::ZkContract,
    function_id: &u32,
    amount: &Amount,
    executor_fees: &mut Vec<Money>,
) -> Result<(zk::ZkVerifierKey, zk::ZkCompressedState), BlockchainError> {
    if let Some(token) = &contract.token {
        /*let mut bal = chain.get_contract_balance(contract_id.clone(), *token_id)?;
        if bal + *amount < bal || token.token.supply + *amount < token.token.supply {
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
        )])?;*/

        let func = token
            .mint_functions
            .get(*function_id as usize)
            .ok_or(BlockchainError::ContractFunctionNotFound)?;
        let circuit = &func.verifier_key;
        let mut state_builder = zk::ZkStateBuilder::<CoreZkHasher>::new(zk::ZkStateModel::Scalar);
        state_builder.batch_set(&zk::ZkDeltaPairs(
            [(zk::ZkDataLocator(vec![]), Some((*amount).into()))].into(),
        ))?;
        let aux_data = state_builder.compress()?;
        Ok((circuit.clone(), aux_data))
    } else {
        Err(BlockchainError::ContractNotToken)
    }
}
