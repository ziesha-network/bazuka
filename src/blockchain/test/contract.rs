use super::*;
use std::str::FromStr;

#[test]
fn test_contract_create_patch() -> Result<(), BlockchainError> {
    let miner = Wallet::new(Vec::from("MINER"));
    let alice = Wallet::new(Vec::from("ABC"));
    let mut chain = KvStoreChain::new(db::RamKvStore::new(), easy_genesis())?;

    let state_model = zk::ZkStateModel::new(1, 10);
    let full_state = zk::ZkState::new(1, state_model, HashMap::new());

    let tx = alice.create_contract(
        zk::ZkContract {
            state_model,
            initial_state: full_state.compress(),
            deposit_withdraw_function: zk::ZkVerifierKey::Dummy,
            functions: Vec::new(),
        },
        full_state.clone(),
        0,
        1,
    );

    let draft = chain.draft_block(1, &[tx.clone()], &miner)?;
    chain.apply_block(&draft.block, true)?;

    assert_eq!(chain.get_height()?, 2);
    assert_eq!(chain.get_outdated_states()?.len(), 1);

    chain.update_states(&draft.patch)?;

    assert_eq!(chain.get_height()?, 2);
    assert_eq!(chain.get_outdated_states()?.len(), 0);

    Ok(())
}

#[test]
fn test_contract_update() -> Result<(), BlockchainError> {
    let miner = Wallet::new(Vec::from("MINER"));
    let alice = Wallet::new(Vec::from("ABC"));
    let cid =
        ContractId::from_str("ac8172140e8ad67651c0be0b14210489d59c51890ef7db60541b3f050247b338")
            .unwrap();
    let mut chain = KvStoreChain::new(db::RamKvStore::new(), easy_genesis())?;

    let state_model = zk::ZkStateModel::new(1, 10);
    let mut full_state = zk::ZkState::new(
        1,
        state_model,
        [(100, zk::ZkScalar::from(200))].into_iter().collect(),
    );
    let state_delta = zk::ZkStateDelta::new([(123, zk::ZkScalar::from(234))].into_iter().collect());
    full_state.apply_delta(&state_delta);

    let tx = alice.call_function(
        cid,
        0,
        state_delta.clone(),
        full_state.compress(),
        zk::ZkProof::Dummy(true),
        0,
        1,
    );

    let draft = chain.draft_block(1, &[tx.clone()], &miner)?;

    chain.apply_block(&draft.block, true)?;

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
                ZkStatePatch::Delta(zk::ZkStateDelta::new(
                    [(123, zk::ZkScalar::from(321))].into_iter().collect()
                ))
            )]
            .into_iter()
            .collect()
        }),
        Err(BlockchainError::FullStateNotValid)
    ));
    chain.fork_on_ram().update_states(&ZkBlockchainPatch {
        patches: [(cid, ZkStatePatch::Delta(state_delta.clone()))]
            .into_iter()
            .collect(),
    })?;
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
                ZkStatePatch::Full(zk::ZkState::new(
                    2,
                    state_model,
                    [(100, zk::ZkScalar::from(200))].into_iter().collect()
                ))
            )]
            .into_iter()
            .collect()
        }),
        Err(BlockchainError::FullStateNotValid)
    ));
    assert!(matches!(
        chain.fork_on_ram().update_states(&ZkBlockchainPatch {
            patches: [(
                cid,
                ZkStatePatch::Full(zk::ZkState::new(
                    1,
                    state_model,
                    [
                        (100, zk::ZkScalar::from(200)),
                        (123, zk::ZkScalar::from(234))
                    ]
                    .into_iter()
                    .collect()
                ))
            )]
            .into_iter()
            .collect()
        }),
        Err(BlockchainError::FullStateNotValid)
    ));
    chain.fork_on_ram().update_states(&ZkBlockchainPatch {
        patches: [(
            cid,
            ZkStatePatch::Full(zk::ZkState::new(
                2,
                state_model,
                [
                    (100, zk::ZkScalar::from(200)),
                    (123, zk::ZkScalar::from(234)),
                ]
                .into_iter()
                .collect(),
            )),
        )]
        .into_iter()
        .collect(),
    })?;

    let mut unupdated_fork = chain.fork_on_ram();
    let mut updated_fork = chain.fork_on_ram();
    updated_fork.update_states(&ZkBlockchainPatch {
        patches: [(
            cid,
            ZkStatePatch::Delta(zk::ZkStateDelta::new(
                [(123, zk::ZkScalar::from(234))].into_iter().collect(),
            )),
        )]
        .into_iter()
        .collect(),
    })?;
    assert_eq!(updated_fork.get_outdated_states()?.len(), 0);
    let updated_tip_hash = updated_fork.get_tip()?.hash();

    let outdated_states = unupdated_fork.get_outdated_states_request()?;
    assert_eq!(outdated_states.len(), 1);

    let gen_state_patch = updated_fork.generate_state_patch(outdated_states, updated_tip_hash)?;
    unupdated_fork.update_states(&gen_state_patch)?;
    assert_eq!(unupdated_fork.get_outdated_states()?.len(), 0);

    chain.update_states(&draft.patch)?;

    assert_eq!(chain.get_height()?, 2);
    assert_eq!(chain.get_outdated_states()?.len(), 0);

    assert!(matches!(
        chain.apply_tx(
            &alice
                .call_function(
                    cid,
                    0,
                    state_delta.clone(),
                    full_state.compress(),
                    zk::ZkProof::Dummy(true),
                    0,
                    1,
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
                    ContractId::from_str(
                        "0000000000000000000000000000000000000000000000000000000000000000"
                    )
                    .unwrap(),
                    0,
                    state_delta.clone(),
                    full_state.compress(),
                    zk::ZkProof::Dummy(true),
                    0,
                    2,
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
                    cid,
                    1,
                    state_delta.clone(),
                    full_state.compress(),
                    zk::ZkProof::Dummy(true),
                    0,
                    2,
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
                    cid,
                    0,
                    state_delta,
                    full_state.compress(),
                    zk::ZkProof::Dummy(false),
                    0,
                    2,
                )
                .tx,
            false
        ),
        Err(BlockchainError::IncorrectZkProof)
    ));

    chain.rollback_block()?;

    assert_eq!(chain.get_height()?, 1);
    assert_eq!(chain.get_outdated_states()?.len(), 0);

    chain.rollback_block()?;

    assert_eq!(chain.get_height()?, 0);
    assert_eq!(chain.get_outdated_states()?.len(), 0);

    Ok(())
}
