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
    /*let miner = TxBuilder::new(&Vec::from("MINER"));
    let alice = TxBuilder::new(&Vec::from("ABC"));
    let bob = TxBuilder::new(&Vec::from("CBA"));

    let mut chain = KvStoreChain::new(db::RamKvStore::new(), easy_config())?;

    // Alice: 10000 Bob: 0
    assert_eq!(
        chain.get_account(alice.get_address())?.balance(TokenId::Ziesha),
        Money(10000)
    );
    assert_eq!(chain.get_account(bob.get_address())?.balance(TokenId::Ziesha), Money(0));*/
    Ok(())
}
