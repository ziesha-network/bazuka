use super::*;

pub fn function_call<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    contract_id: &ContractId,
    contract: &zk::ZkContract,
    function_id: &u32,
    fee: &Money,
    executor_fees: &mut Vec<Money>,
) -> Result<(zk::ZkVerifierKey, zk::ZkCompressedState), BlockchainError> {
    executor_fees.push(*fee);

    let mut cont_balance = chain.get_contract_balance(*contract_id, fee.token_id)?;
    if cont_balance < fee.amount {
        return Err(BlockchainError::BalanceInsufficient);
    }
    cont_balance -= fee.amount;
    chain.database.update(&[WriteOp::Put(
        keys::contract_balance(contract_id, fee.token_id),
        cont_balance.into(),
    )])?;

    let func = contract
        .functions
        .get(*function_id as usize)
        .ok_or(BlockchainError::ContractFunctionNotFound)?;
    let circuit = &func.verifier_key;
    let mut state_builder = zk::ZkStateBuilder::<CoreZkHasher>::new(zk::ZkStateModel::Struct {
        field_types: vec![
            zk::ZkStateModel::Scalar, // Token-id
            zk::ZkStateModel::Scalar, // Total fee
        ],
    });
    state_builder.batch_set(&zk::ZkDeltaPairs(
        [
            (zk::ZkDataLocator(vec![0]), Some(fee.token_id.into())),
            (
                zk::ZkDataLocator(vec![1]),
                Some(zk::ZkScalar::from(fee.amount)),
            ),
        ]
        .into(),
    ))?;
    let aux_data = state_builder.compress()?;
    Ok((circuit.clone(), aux_data))
}
