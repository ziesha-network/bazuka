use super::*;

#[test]
fn test_token_balances() -> Result<(), BlockchainError> {
    let miner = TxBuilder::new(&Vec::from("VALIDATOR"));
    let alice = TxBuilder::new(&Vec::from("ABCD"));
    let bob = TxBuilder::new(&Vec::from("DCBA"));

    let mut chain = KvStoreChain::new(
        db::RamKvStore::new(),
        blockchain::get_test_blockchain_config(),
    )?;

    let (token_create_tx, token_id) = alice.create_token(
        "".into(),
        "My Token".into(),
        "MYT".into(),
        Amount(12345),
        0,
        Some(alice.get_address()),
        Money::ziesha(0),
        1,
    );
    assert_eq!(chain.get_balance(alice.get_address(), token_id)?, Amount(0));

    // Cannot spend uncreated token
    assert!(matches!(
        chain.draft_block(
            1,
            &[alice.create_transaction(
                "".into(),
                bob.get_address(),
                Money::new(token_id, 1),
                Money::ziesha(0),
                1
            )],
            &miner,
            false,
        ),
        Err(BlockchainError::BalanceInsufficient)
    ));

    chain.apply_block(
        &chain
            .draft_block(1, &[token_create_tx], &miner, false)?
            .unwrap()
            .block,
    )?;

    assert_eq!(
        chain.get_balance(alice.get_address(), token_id)?,
        Amount(12345)
    );

    chain.apply_block(
        &chain
            .draft_block(
                1,
                &[alice.create_transaction(
                    "".into(),
                    bob.get_address(),
                    Money::new(token_id, 20),
                    Money::ziesha(0),
                    2,
                )],
                &miner,
                false,
            )?
            .unwrap()
            .block,
    )?;

    assert_eq!(
        chain.get_balance(alice.get_address(), token_id)?,
        Amount(12325)
    );
    assert_eq!(chain.get_balance(bob.get_address(), token_id)?, Amount(20));

    // Check insufficient token balance
    assert!(matches!(
        chain.draft_block(
            1,
            &[alice.create_transaction(
                "".into(),
                bob.get_address(),
                Money::new(token_id, 12326),
                Money::ziesha(0),
                3
            )],
            &miner,
            false,
        ),
        Err(BlockchainError::BalanceInsufficient)
    ));

    chain.apply_block(
        &chain
            .draft_block(
                1,
                &[alice.create_transaction(
                    "".into(),
                    bob.get_address(),
                    Money::new(token_id, 12325),
                    Money::ziesha(0),
                    3,
                )],
                &miner,
                false,
            )?
            .unwrap()
            .block,
    )?;

    assert_eq!(chain.get_balance(alice.get_address(), token_id)?, Amount(0));

    rollback_till_empty(&mut chain)?;

    Ok(())
}
