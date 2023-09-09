use super::*;

pub fn create_contract<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    tx_src: Address,
    contract_id: ContractId,
    contract: &zk::ZkContract,
    state: &Option<zk::ZkDataPairs>,
    money: Money,
) -> Result<(), BlockchainError> {
    if !contract.state_model.is_valid::<CoreZkHasher>() {
        return Err(BlockchainError::InvalidStateModel);
    }
    if let Some(token) = &contract.token {
        if !token.token.validate() {
            return Err(BlockchainError::TokenBadNameSymbol);
        }
        chain.database.update(&[WriteOp::Put(
            keys::account_balance(&tx_src, contract_id),
            token.token.supply.into(),
        )])?;
        chain.database.update(&[WriteOp::Put(
            keys::token(&contract_id),
            (&token.token).into(),
        )])?;
    }
    chain.database.update(&[WriteOp::Put(
        keys::contract(&contract_id),
        contract.clone().into(),
    )])?;
    chain.database.update(&[WriteOp::Put(
        keys::contract_account(&contract_id),
        ContractAccount {
            compressed_state: contract.initial_state,
            height: 1,
        }
        .into(),
    )])?;
    let state = state.clone().ok_or(BlockchainError::StateNotGiven)?;
    if contract_id == chain.config().mpn_config.mpn_contract_id {
        index_mpn_accounts(chain, &state.as_delta())?;
    }

    zk::KvStoreStateManager::<CoreZkHasher>::update_contract(
        &mut chain.database,
        contract_id,
        &state.as_delta(),
        1,
    )?;
    if zk::KvStoreStateManager::<CoreZkHasher>::root(&mut chain.database, contract_id)?
        != contract.initial_state
    {
        return Err(BlockchainError::InvalidState);
    }

    // Deposit initial balance to the contract
    let mut src_bal = chain.get_balance(tx_src.clone(), money.token_id)?;
    if src_bal < money.amount {
        return Err(BlockchainError::BalanceInsufficient);
    }
    src_bal -= money.amount;
    chain.database.update(&[WriteOp::Put(
        keys::account_balance(&tx_src, money.token_id),
        src_bal.into(),
    )])?;
    let mut dst_bal = chain.get_contract_balance(contract_id.clone(), money.token_id)?;
    dst_bal += money.amount;
    chain.database.update(&[WriteOp::Put(
        keys::contract_balance(&contract_id, money.token_id),
        dst_bal.into(),
    )])?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{RamKvStore, WriteOp};

    #[test]
    fn test_create_contract() {
        let abc = TxBuilder::new(&Vec::from("ABC"));
        let chain = KvStoreChain::new(
            RamKvStore::new(),
            crate::config::blockchain::get_test_blockchain_config(),
        )
        .unwrap();
        let contract_id: ContractId =
            "0x0001020304050607080900010203040506070809000102030405060708090001"
                .parse()
                .unwrap();
        let state_model = zk::ZkStateModel::Struct {
            field_types: vec![zk::ZkStateModel::Scalar, zk::ZkStateModel::Scalar],
        };
        let initial_state =
            zk::ZkCompressedState::empty::<crate::core::ZkHasher>(state_model.clone());
        let contract = zk::ZkContract {
            token: None,
            state_model,
            initial_state: initial_state.clone(),
            deposit_functions: vec![],
            withdraw_functions: vec![],
            functions: vec![],
        };
        let (ops, _) = chain
            .isolated(|chain| {
                Ok(create_contract(
                    chain,
                    abc.get_address(),
                    contract_id,
                    &contract,
                    &Some(Default::default()),
                    Money::ziesha(2345),
                )?)
            })
            .unwrap();

        let expected_ops = vec![
            WriteOp::Put(
                "ACB-ed8c19c6a4cf1460e961f7bae8eea54d437b9edac27cbeb09be32ae367adf9098a-Ziesha"
                    .into(),
                7655u64.into(),
            ),
            WriteOp::Put(
                "CAB-0x0001020304050607080900010203040506070809000102030405060708090001-Ziesha"
                    .into(),
                2345u64.into(),
            ),
            WriteOp::Put(
                "CAC-0x0001020304050607080900010203040506070809000102030405060708090001".into(),
                ContractAccount {
                    height: 1,
                    compressed_state: initial_state.clone(),
                }
                .into(),
            ),
            WriteOp::Put(
                "CON-0x0001020304050607080900010203040506070809000102030405060708090001".into(),
                contract.into(),
            ),
            WriteOp::Put(
                "S-0x0001020304050607080900010203040506070809000102030405060708090001-HGT".into(),
                1u64.into(),
            ),
            WriteOp::Put(
                "S-0x0001020304050607080900010203040506070809000102030405060708090001-RT".into(),
                initial_state.into(),
            ),
        ];

        assert_eq!(ops, expected_ops);
    }
}
