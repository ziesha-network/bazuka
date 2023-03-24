use super::*;

pub fn deposit<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    contract_id: &ContractId,
    contract: &zk::ZkContract,
    deposit_circuit_id: &u32,
    deposits: &[ContractDeposit],
    executor_fees: &mut Vec<Money>,
) -> Result<(zk::ZkVerifierKey, zk::ZkCompressedState), BlockchainError> {
    let deposit_func = contract
        .deposit_functions
        .get(*deposit_circuit_id as usize)
        .ok_or(BlockchainError::ContractFunctionNotFound)?;
    let circuit = &deposit_func.verifier_key;
    let state_model = zk::ZkStateModel::List {
        item_type: Box::new(zk::ZkStateModel::Struct {
            field_types: vec![
                zk::ZkStateModel::Scalar, // Enabled
                zk::ZkStateModel::Scalar, // Token-id
                zk::ZkStateModel::Scalar, // Amount
                zk::ZkStateModel::Scalar, // Calldata
            ],
        }),
        log4_size: deposit_func.log4_payment_capacity,
    };
    let mut state_builder = zk::ZkStateBuilder::<CoreZkHasher>::new(state_model);
    for (i, deposit) in deposits.iter().enumerate() {
        if deposit.contract_id != *contract_id || deposit.deposit_circuit_id != *deposit_circuit_id
        {
            return Err(BlockchainError::DepositWithdrawPassedToWrongFunction);
        }
        executor_fees.push(deposit.fee);
        state_builder.batch_set(&zk::ZkDeltaPairs(
            [
                (
                    zk::ZkDataLocator(vec![i as u64, 0]),
                    Some(zk::ZkScalar::from(1)),
                ),
                (
                    zk::ZkDataLocator(vec![i as u64, 1]),
                    Some(deposit.amount.token_id.into()),
                ),
                (
                    zk::ZkDataLocator(vec![i as u64, 2]),
                    Some(zk::ZkScalar::from(deposit.amount.amount)),
                ),
                (zk::ZkDataLocator(vec![i as u64, 3]), Some(deposit.calldata)),
            ]
            .into(),
        ))?;

        chain.apply_deposit(deposit)?;
    }
    let aux_data = state_builder.compress()?;
    Ok((circuit.clone(), aux_data))
}
