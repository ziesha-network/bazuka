use super::*;

pub fn withdraw<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    contract_id: &ContractId,
    contract: &zk::ZkContract,
    withdraw_circuit_id: &u32,
    withdraws: &[ContractWithdraw],
    executor_fees: &mut Vec<Money>,
) -> Result<(zk::ZkVerifierKey, zk::ZkCompressedState), BlockchainError> {
    let withdraw_func = contract
        .withdraw_functions
        .get(*withdraw_circuit_id as usize)
        .ok_or(BlockchainError::ContractFunctionNotFound)?;
    let circuit = &withdraw_func.verifier_key;
    let state_model = zk::ZkStateModel::List {
        item_type: Box::new(zk::ZkStateModel::Struct {
            field_types: vec![
                zk::ZkStateModel::Scalar, // Enabled
                zk::ZkStateModel::Scalar, // Amount token-id
                zk::ZkStateModel::Scalar, // Amount
                zk::ZkStateModel::Scalar, // Fee token-id
                zk::ZkStateModel::Scalar, // Fee
                zk::ZkStateModel::Scalar, // Fingerprint
                zk::ZkStateModel::Scalar, // Calldata
            ],
        }),
        log4_size: withdraw_func.log4_payment_capacity,
    };
    let mut state_builder = zk::ZkStateBuilder::<CoreZkHasher>::new(state_model);
    for (i, withdraw) in withdraws.iter().enumerate() {
        if withdraw.contract_id != *contract_id
            || withdraw.withdraw_circuit_id != *withdraw_circuit_id
        {
            return Err(BlockchainError::DepositWithdrawPassedToWrongFunction);
        }
        let fingerprint = withdraw.fingerprint();
        executor_fees.push(withdraw.fee);
        state_builder.batch_set(&zk::ZkDeltaPairs(
            [
                (
                    zk::ZkDataLocator(vec![i as u64, 0]),
                    Some(zk::ZkScalar::from(1)),
                ),
                (
                    zk::ZkDataLocator(vec![i as u64, 1]),
                    Some(withdraw.amount.token_id.into()),
                ),
                (
                    zk::ZkDataLocator(vec![i as u64, 2]),
                    Some(zk::ZkScalar::from(withdraw.amount.amount)),
                ),
                (
                    zk::ZkDataLocator(vec![i as u64, 3]),
                    Some(withdraw.fee.token_id.into()),
                ),
                (
                    zk::ZkDataLocator(vec![i as u64, 4]),
                    Some(zk::ZkScalar::from(withdraw.fee.amount)),
                ),
                (zk::ZkDataLocator(vec![i as u64, 5]), Some(fingerprint)),
                (
                    zk::ZkDataLocator(vec![i as u64, 6]),
                    Some(withdraw.calldata),
                ),
            ]
            .into(),
        ))?;

        chain.apply_withdraw(withdraw)?;
    }
    let aux_data = state_builder.compress()?;
    Ok((circuit.clone(), aux_data))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{RamKvStore, WriteOp};

    #[test]
    fn test_empty_withdraw() {
        let chain = KvStoreChain::new(
            RamKvStore::new(),
            crate::config::blockchain::get_test_blockchain_config(),
        )
        .unwrap();

        let withdraw_state_model = zk::ZkStateModel::List {
            item_type: Box::new(zk::ZkStateModel::Struct {
                field_types: vec![
                    zk::ZkStateModel::Scalar, // Enabled
                    zk::ZkStateModel::Scalar, // Amount token-id
                    zk::ZkStateModel::Scalar, // Amount
                    zk::ZkStateModel::Scalar, // Fee token-id
                    zk::ZkStateModel::Scalar, // Fee
                    zk::ZkStateModel::Scalar, // Fingerprint
                    zk::ZkStateModel::Scalar, // Calldata
                ],
            }),
            log4_size: 1,
        };
        let empty_compressed =
            zk::ZkCompressedState::empty::<crate::core::ZkHasher>(withdraw_state_model);

        let (ops, ((_, aux_data), exec_fees)) = chain
            .isolated(|chain| {
                let mut exec_fees = Vec::new();
                let contract_id = chain.config.mpn_config.mpn_contract_id;
                let contract = chain.get_contract(contract_id)?;
                Ok((
                    withdraw(chain, &contract_id, &contract, &0, &[], &mut exec_fees)?,
                    exec_fees,
                ))
            })
            .unwrap();

        assert_eq!(aux_data, empty_compressed);
        assert!(ops.is_empty());
        assert!(exec_fees.is_empty());
    }
}
