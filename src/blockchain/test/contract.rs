use super::*;

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
        Money::ziesha(0),
        1,
    );

    let draft = chain
        .draft_block(100, &[tx.clone()], &miner, true)
        .unwrap()
        .unwrap();
    chain.apply_block(&draft).unwrap();

    assert_eq!(chain.get_height().unwrap(), 2);

    rollback_till_empty(&mut chain).unwrap();
}
