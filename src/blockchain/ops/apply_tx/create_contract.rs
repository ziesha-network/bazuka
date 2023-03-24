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
    use crate::db::{RamKvStore, WriteOp};

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
        let initial_state =
            zk::ZkCompressedState::empty::<crate::core::ZkHasher>(state_model.clone());
        let contract = zk::ZkContract {
            state_model,
            initial_state: initial_state.clone(),
            deposit_functions: vec![],
            withdraw_functions: vec![],
            functions: vec![],
        };
        let (ops, out) = chain
            .isolated(|chain| Ok(create_contract(chain, contract_id, &contract)?))
            .unwrap();

        assert_eq!(
            out,
            TxSideEffect::StateChange {
                contract_id,
                state_change: ZkCompressedStateChange {
                    prev_height: 0,
                    prev_state: initial_state.clone(),
                    state: initial_state.clone(),
                },
            }
        );

        let expected_ops = vec![
            WriteOp::Put(
                "CAC-0001020304050607080900010203040506070809000102030405060708090001".into(),
                ContractAccount {
                    height: 1,
                    compressed_state: initial_state.clone(),
                }
                .into(),
            ),
            WriteOp::Put(
                "CON-0001020304050607080900010203040506070809000102030405060708090001".into(),
                contract.into(),
            ),
            WriteOp::Put(
                "CSA-0000000001-0001020304050607080900010203040506070809000102030405060708090001"
                    .into(),
                initial_state.into(),
            ),
        ];

        assert_eq!(ops, expected_ops);
    }
}
