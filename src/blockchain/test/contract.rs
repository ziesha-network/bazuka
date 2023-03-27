use super::*;
use std::str::FromStr;

#[test]
fn test_contract_create_patch() {
    let miner = TxBuilder::new(&Vec::from("VALIDATOR"));
    let alice = TxBuilder::new(&Vec::from("ABC"));
    let mut chain = KvStoreChain::new(
        db::RamKvStore::new(),
        blockchain::get_test_blockchain_config(),
    )
    .unwrap();

    let state_model = zk::ZkStateModel::List {
        item_type: Box::new(zk::ZkStateModel::Scalar),
        log4_size: 5,
    };
    let full_state = zk::ZkState {
        rollbacks: vec![],
        data: Default::default(),
    };

    let tx = alice.create_contract(
        "".into(),
        zk::ZkContract {
            state_model: state_model.clone(),
            initial_state: state_model
                .compress::<CoreZkHasher>(&full_state.data)
                .unwrap(),
            deposit_functions: Vec::new(),
            withdraw_functions: Vec::new(),
            functions: Vec::new(),
        },
        full_state.data.clone(),
        Money::ziesha(0),
        1,
    );

    let draft = chain
        .draft_block(1, &[tx.clone()], &miner, true)
        .unwrap()
        .unwrap();
    chain.apply_block(&draft.block).unwrap();

    assert_eq!(chain.get_height().unwrap(), 2);
    assert_eq!(chain.get_outdated_contracts().unwrap().len(), 1);

    chain.update_states(&draft.patch).unwrap();

    assert_eq!(chain.get_height().unwrap(), 2);
    assert_eq!(chain.get_outdated_contracts().unwrap().len(), 0);

    rollback_till_empty(&mut chain).unwrap();
}

#[test]
fn test_mpn_contract_patching() {
    let miner = TxBuilder::new(&Vec::from("VALIDATOR"));
    let alice = TxBuilder::new(&Vec::from("ABC"));

    let mut chain = KvStoreChain::new(
        db::RamKvStore::new(),
        blockchain::get_test_blockchain_config(),
    )
    .unwrap();

    let cid = chain.config().mpn_config.mpn_contract_id;

    let state_model = chain.config().mpn_config.state_model();
    let mut full_state = zk::ZkState {
        rollbacks: vec![],
        data: zk::ZkDataPairs(
            [(zk::ZkDataLocator(vec![1, 0]), zk::ZkScalar::from(200))]
                .into_iter()
                .collect(),
        ),
    };

    let init_tx = alice.call_function(
        "".into(),
        cid,
        0,
        full_state.data.as_delta(),
        state_model
            .compress::<CoreZkHasher>(&full_state.data)
            .unwrap(),
        zk::ZkProof::Dummy(true),
        Money::ziesha(0),
        Money::ziesha(0),
        1,
    );
    let draft = chain
        .draft_block(1, &[init_tx.clone()], &miner, false)
        .unwrap()
        .unwrap();
    chain.apply_block(&draft.block).unwrap();
    chain
        .update_states(&ZkBlockchainPatch {
            patches: [(
                cid,
                zk::ZkStatePatch::Delta(zk::ZkDeltaPairs(
                    [(zk::ZkDataLocator(vec![1, 0]), Some(zk::ZkScalar::from(200)))]
                        .into_iter()
                        .collect(),
                )),
            )]
            .into_iter()
            .collect(),
        })
        .unwrap();

    let mut full_state_with_delta = full_state.clone();

    let state_delta = zk::ZkDeltaPairs(
        [(
            zk::ZkDataLocator(vec![2, 3, 1, 0]),
            Some(zk::ZkScalar::from(234)),
        )]
        .into_iter()
        .collect(),
    );
    full_state.apply_delta(&state_delta);
    full_state_with_delta.push_delta(&state_delta);

    let tx = alice.call_function(
        "".into(),
        cid,
        0,
        state_delta.clone(),
        state_model
            .compress::<CoreZkHasher>(&full_state.data)
            .unwrap(),
        zk::ZkProof::Dummy(true),
        Money::ziesha(0),
        Money::ziesha(0),
        2,
    );
    let draft = chain
        .draft_block(2, &[tx.clone()], &miner, false)
        .unwrap()
        .unwrap();
    chain.apply_block(&draft.block).unwrap();

    assert!(matches!(
        chain.fork_on_ram().update_states(&ZkBlockchainPatch {
            patches: HashMap::new()
        }),
        Err(BlockchainError::FullStateNotFound)
    ));
    assert!(matches!(
        chain.fork_on_ram().update_states(&ZkBlockchainPatch {
            patches: [(
                cid,
                zk::ZkStatePatch::Delta(zk::ZkDeltaPairs(
                    [(
                        zk::ZkDataLocator(vec![2, 3, 1, 0]),
                        Some(zk::ZkScalar::from(321))
                    )]
                    .into_iter()
                    .collect()
                ))
            )]
            .into_iter()
            .collect()
        }),
        Err(BlockchainError::FullStateNotValid)
    ));
    chain
        .fork_on_ram()
        .update_states(&ZkBlockchainPatch {
            patches: [(cid, zk::ZkStatePatch::Delta(state_delta.clone()))]
                .into_iter()
                .collect(),
        })
        .unwrap();
    assert!(matches!(
        chain.fork_on_ram().update_states(&ZkBlockchainPatch {
            patches: HashMap::new()
        }),
        Err(BlockchainError::FullStateNotFound)
    ));
    assert!(matches!(
        chain.fork_on_ram().update_states(&ZkBlockchainPatch {
            patches: [(
                cid,
                zk::ZkStatePatch::Full(zk::ZkState {
                    rollbacks: vec![],
                    data: zk::ZkDataPairs(
                        [(zk::ZkDataLocator(vec![1, 0]), zk::ZkScalar::from(200))]
                            .into_iter()
                            .collect()
                    )
                })
            )]
            .into_iter()
            .collect()
        }),
        Err(BlockchainError::FullStateNotValid)
    ));
    chain
        .fork_on_ram()
        .update_states(&ZkBlockchainPatch {
            patches: [(
                cid,
                zk::ZkStatePatch::Full(zk::ZkState {
                    rollbacks: vec![],
                    data: zk::ZkDataPairs(
                        [
                            (zk::ZkDataLocator(vec![1, 0]), zk::ZkScalar::from(200)),
                            (zk::ZkDataLocator(vec![2, 3, 1, 0]), zk::ZkScalar::from(234)),
                        ]
                        .into_iter()
                        .collect(),
                    ),
                }),
            )]
            .into_iter()
            .collect(),
        })
        .unwrap();
    chain
        .fork_on_ram()
        .update_states(&ZkBlockchainPatch {
            patches: [(
                cid,
                zk::ZkStatePatch::Full(zk::ZkState {
                    rollbacks: vec![],
                    data: zk::ZkDataPairs(
                        [
                            (zk::ZkDataLocator(vec![1, 0]), zk::ZkScalar::from(200)),
                            (zk::ZkDataLocator(vec![2, 3, 1, 0]), zk::ZkScalar::from(234)),
                        ]
                        .into_iter()
                        .collect(),
                    ),
                }),
            )]
            .into_iter()
            .collect(),
        })
        .unwrap();
    chain
        .fork_on_ram()
        .update_states(&ZkBlockchainPatch {
            patches: [(cid, zk::ZkStatePatch::Full(full_state_with_delta))]
                .into_iter()
                .collect(),
        })
        .unwrap();
    let mut unupdated_fork = chain.fork_on_ram();
    let mut updated_fork = chain.fork_on_ram();
    updated_fork
        .update_states(&ZkBlockchainPatch {
            patches: [(
                cid,
                zk::ZkStatePatch::Delta(zk::ZkDeltaPairs(
                    [(
                        zk::ZkDataLocator(vec![2, 3, 1, 0]),
                        Some(zk::ZkScalar::from(234)),
                    )]
                    .into_iter()
                    .collect(),
                )),
            )]
            .into_iter()
            .collect(),
        })
        .unwrap();
    assert_eq!(updated_fork.get_outdated_contracts().unwrap().len(), 0);
    let updated_tip_hash = updated_fork.get_tip().unwrap().hash();

    let outdated_heights = unupdated_fork.get_outdated_heights().unwrap();
    assert_eq!(outdated_heights.len(), 1);

    let gen_state_patch = updated_fork
        .generate_state_patch(outdated_heights, updated_tip_hash)
        .unwrap();
    unupdated_fork.update_states(&gen_state_patch).unwrap();
    assert_eq!(unupdated_fork.get_outdated_contracts().unwrap().len(), 0);
    chain.update_states(&draft.patch).unwrap();

    assert_eq!(chain.get_height().unwrap(), 3);
    assert_eq!(chain.get_outdated_contracts().unwrap().len(), 0);

    assert!(matches!(
        chain.apply_tx(
            &alice
                .call_function(
                    "".into(),
                    cid,
                    0,
                    state_delta.clone(),
                    state_model
                        .compress::<CoreZkHasher>(&full_state.data)
                        .unwrap(),
                    zk::ZkProof::Dummy(true),
                    Money::ziesha(0),
                    Money::ziesha(0),
                    2,
                )
                .tx,
            false
        ),
        Err(BlockchainError::InvalidTransactionNonce)
    ));

    assert!(matches!(
        chain.apply_tx(
            &alice
                .call_function(
                    "".into(),
                    ContractId::from_str(
                        "0000000000000000000000000000000000000000000000000000000000000000"
                    )
                    .unwrap(),
                    0,
                    state_delta.clone(),
                    state_model
                        .compress::<CoreZkHasher>(&full_state.data)
                        .unwrap(),
                    zk::ZkProof::Dummy(true),
                    Money::ziesha(0),
                    Money::ziesha(0),
                    3,
                )
                .tx,
            false
        ),
        Err(BlockchainError::ContractNotFound)
    ));

    assert!(matches!(
        chain.apply_tx(
            &alice
                .call_function(
                    "".into(),
                    cid,
                    1,
                    state_delta.clone(),
                    state_model
                        .compress::<CoreZkHasher>(&full_state.data)
                        .unwrap(),
                    zk::ZkProof::Dummy(true),
                    Money::ziesha(0),
                    Money::ziesha(0),
                    3,
                )
                .tx,
            false
        ),
        Err(BlockchainError::ContractFunctionNotFound)
    ));

    assert!(matches!(
        chain.apply_tx(
            &alice
                .call_function(
                    "".into(),
                    cid,
                    0,
                    state_delta,
                    state_model
                        .compress::<CoreZkHasher>(&full_state.data)
                        .unwrap(),
                    zk::ZkProof::Dummy(false),
                    Money::ziesha(0),
                    Money::ziesha(0),
                    3,
                )
                .tx,
            false
        ),
        Err(BlockchainError::IncorrectZkProof)
    ));

    chain.rollback().unwrap();

    assert_eq!(chain.get_height().unwrap(), 2);
    assert_eq!(chain.get_outdated_contracts().unwrap().len(), 0);

    chain.rollback().unwrap();

    assert_eq!(chain.get_height().unwrap(), 1);
    assert_eq!(chain.get_outdated_contracts().unwrap().len(), 0);

    chain.rollback().unwrap();

    assert_eq!(chain.get_height().unwrap(), 0);
    assert_eq!(chain.get_outdated_contracts().unwrap().len(), 0);
}
