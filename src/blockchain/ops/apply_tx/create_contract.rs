use super::*;

pub fn create_contract<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    contract_id: ContractId,
    contract: &zk::ZkContract,
) -> Result<TxSideEffect, BlockchainError> {
    if !contract.state_model.is_valid::<CoreZkHasher>() {
        return Err(BlockchainError::InvalidStateModel);
    }
    chain.database.update(&[WriteOp::Put(
        keys::contract(&contract_id),
        contract.clone().into(),
    )])?;
    let compressed_empty =
        zk::ZkCompressedState::empty::<CoreZkHasher>(contract.state_model.clone());
    chain.database.update(&[WriteOp::Put(
        keys::contract_account(&contract_id),
        ContractAccount {
            compressed_state: contract.initial_state,
            height: 1,
        }
        .into(),
    )])?;
    chain.database.update(&[WriteOp::Put(
        keys::compressed_state_at(&contract_id, 1),
        contract.initial_state.into(),
    )])?;
    Ok(TxSideEffect::StateChange {
        contract_id,
        state_change: ZkCompressedStateChange {
            prev_height: 0,
            prev_state: compressed_empty,
            state: contract.initial_state,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Blob, RamKvStore, StringKey, WriteOp};

    #[test]
    fn test_create_contract() {
        let chain = KvStoreChain::new(
            RamKvStore::new(),
            crate::config::blockchain::get_test_blockchain_config(),
        )
        .unwrap();
        let contract_id: ContractId =
            "0001020304050607080900010203040506070809000102030405060708090001"
                .parse()
                .unwrap();
        let state_model = zk::ZkStateModel::Struct {
            field_types: vec![zk::ZkStateModel::Scalar, zk::ZkStateModel::Scalar],
        };
        let contract = zk::ZkContract {
            state_model: state_model.clone(),
            initial_state: zk::ZkCompressedState::empty::<crate::core::ZkHasher>(state_model),
            deposit_functions: vec![],
            withdraw_functions: vec![],
            functions: vec![],
        };
        let (ops, _) = chain
            .isolated(|chain| Ok(create_contract(chain, contract_id, &contract)?))
            .unwrap();

        let expected_ops = vec![
            WriteOp::Put(
                "CAC-0001020304050607080900010203040506070809000102030405060708090001".into(),
                Blob(vec![
                    1, 0, 0, 0, 0, 0, 0, 0, 3, 77, 215, 124, 110, 70, 210, 216, 59, 110, 151, 11,
                    124, 38, 215, 74, 19, 103, 181, 50, 200, 82, 47, 185, 15, 121, 235, 179, 112,
                    246, 179, 106, 0, 0, 0, 0, 0, 0, 0, 0,
                ]),
            ),
            WriteOp::Put(
                "CON-0001020304050607080900010203040506070809000102030405060708090001".into(),
                Blob(vec![
                    3, 77, 215, 124, 110, 70, 210, 216, 59, 110, 151, 11, 124, 38, 215, 74, 19,
                    103, 181, 50, 200, 82, 47, 185, 15, 121, 235, 179, 112, 246, 179, 106, 0, 0, 0,
                    0, 0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                ]),
            ),
            WriteOp::Put(
                "CSA-0000000001-0001020304050607080900010203040506070809000102030405060708090001"
                    .into(),
                Blob(vec![
                    3, 77, 215, 124, 110, 70, 210, 216, 59, 110, 151, 11, 124, 38, 215, 74, 19,
                    103, 181, 50, 200, 82, 47, 185, 15, 121, 235, 179, 112, 246, 179, 106, 0, 0, 0,
                    0, 0, 0, 0, 0,
                ]),
            ),
        ];

        assert_eq!(ops, expected_ops);
    }
}
