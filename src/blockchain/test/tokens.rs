use super::*;
use crate::zk::ZkScalar;
#[test]
fn test_disallow_duplicate_token() -> Result<(), BlockchainError> {
    let miner = TxBuilder::new(&Vec::from("MINER"));
    let alice = TxBuilder::new(&Vec::from("ABC"));

    let mut chain = KvStoreChain::new(db::RamKvStore::new(), easy_config())?;

    // Check disallow recreating Ziesha token
    assert!(matches!(
        chain.draft_block(
            3,
            &[alice.create_token(
                TokenId::Ziesha,
                "Ziesha".into(),
                "ZSH".into(),
                Money(200),
                Some(alice.get_address()),
                Money(0),
                1,
            )],
            &miner,
            false,
        ),
        Err(BlockchainError::TokenAlreadyExists)
    ));

    chain.apply_block(
        &chain
            .draft_block(
                1,
                &[alice.create_token(
                    TokenId::Custom(ZkScalar::from(1)),
                    "Token One".into(),
                    "ONE".into(),
                    Money(100),
                    Some(alice.get_address()),
                    Money(0),
                    1,
                )],
                &miner,
                false,
            )?
            .unwrap()
            .block,
        true,
    )?;
    chain.apply_block(
        &chain
            .draft_block(
                2,
                &[alice.create_token(
                    TokenId::Custom(ZkScalar::from(2)),
                    "Token Two".into(),
                    "TWO".into(),
                    Money(200),
                    Some(alice.get_address()),
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

    // Check disallow duplicated tokens
    assert!(matches!(
        chain.draft_block(
            3,
            &[alice.create_token(
                TokenId::Custom(ZkScalar::from(2)),
                "Token Two".into(),
                "TWO".into(),
                Money(200),
                Some(alice.get_address()),
                Money(0),
                3,
            )],
            &miner,
            false,
        ),
        Err(BlockchainError::TokenAlreadyExists)
    ));

    Ok(())
}

#[test]
fn test_token_balances() -> Result<(), BlockchainError> {
    let miner = TxBuilder::new(&Vec::from("MINER"));
    let alice = TxBuilder::new(&Vec::from("ABCD"));
    let bob = TxBuilder::new(&Vec::from("DCBA"));

    let mut chain = KvStoreChain::new(db::RamKvStore::new(), easy_config())?;

    let token_id = TokenId::Custom(ZkScalar::from(123));
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
            .draft_block(
                1,
                &[alice.create_token(
                    token_id,
                    "My Token".into(),
                    "MYT".into(),
                    Money(12345),
                    Some(alice.get_address()),
                    Money(0),
                    1,
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
