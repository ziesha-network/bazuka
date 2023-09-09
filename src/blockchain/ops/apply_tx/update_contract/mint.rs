use super::*;

pub fn mint<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    contract_id: &ContractId,
    contract: &zk::ZkContract,
    function_id: &u32,
    amount: &Amount,
    executor_fees: &mut Vec<Money>,
) -> Result<(zk::ZkVerifierKey, zk::ZkCompressedState), BlockchainError> {
    if let Some(mut token) = contract.token.clone() {
        let mut bal = chain.get_contract_balance(contract_id.clone(), *contract_id)?;
        if bal + *amount < bal || token.token.supply + *amount < token.token.supply {
            return Err(BlockchainError::TokenSupplyOverflow);
        }
        bal += *amount;
        token.token.supply += *amount;
        chain.database.update(&[WriteOp::Put(
            keys::token(contract_id),
            (&token.token).into(),
        )])?;
        chain.database.update(&[WriteOp::Put(
            keys::contract_balance(contract_id, *contract_id),
            bal.into(),
        )])?;

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

        executor_fees.push(Money {
            token_id: contract_id.clone(),
            amount: *amount,
        });

        Ok((circuit.clone(), aux_data))
    } else {
        Err(BlockchainError::ContractNotToken)
    }
}
