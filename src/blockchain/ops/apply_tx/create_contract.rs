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
