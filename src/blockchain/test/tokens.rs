use super::*;

#[test]
fn test_token_balances() -> Result<(), BlockchainError> {
    let miner = TxBuilder::new(&Vec::from("MINER"));
    let alice = TxBuilder::new(&Vec::from("ABCD"));
    let bob = TxBuilder::new(&Vec::from("DCBA"));

    let mut chain = KvStoreChain::new(db::RamKvStore::new(), easy_config())?;

    let (token_create_tx, token_id) = alice.create_token(
        "My Token".into(),
        "MYT".into(),
        Money(12345),
        Some(alice.get_address()),
        Money(0),
        1,
    );
    assert_eq!(chain.get_balance(alice.get_address(), token_id)?, Money(0));

    // Cannot spend uncreated token
    assert!(matches!(
        chain.draft_block(
            1,
            &[alice.create_token_transaction(bob.get_address(), token_id, Money(1), Money(0), 1)],
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
        true,
    )?;

    assert_eq!(
        chain.get_balance(alice.get_address(), token_id)?,
        Money(12345)
    );

    chain.apply_block(
        &chain
            .draft_block(
                1,
                &[alice.create_token_transaction(
                    bob.get_address(),
                    token_id,
                    Money(20),
                    Money(0),
                    2,
                )],
                &miner,
                false,
            )?
            .unwrap()
            .block,
        true,
    )?;

    assert_eq!(
        chain.get_balance(alice.get_address(), token_id)?,
        Money(12325)
    );
    assert_eq!(chain.get_balance(bob.get_address(), token_id)?, Money(20));

    // Check insufficient token balance
    assert!(matches!(
        chain.draft_block(
            1,
            &[alice.create_token_transaction(
                bob.get_address(),
                token_id,
                Money(12326),
                Money(0),
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
                &[alice.create_token_transaction(
                    bob.get_address(),
                    token_id,
                    Money(12325),
                    Money(0),
                    3,
                )],
                &miner,
                false,
            )?
            .unwrap()
            .block,
        true,
    )?;

    assert_eq!(chain.get_balance(alice.get_address(), token_id)?, Money(0));

    rollback_till_empty(&mut chain)?;

    Ok(())
}
