use super::*;
use crate::config::blockchain;
use crate::core::{Hasher, Signature, Signer, TransactionData};
use crate::crypto::SignatureScheme;
use crate::db;

mod contract;
mod rewards;
mod tokens;
mod vrf_randomness;

fn rollback_till_empty<K: KvStore>(b: &mut KvStoreChain<K>) -> Result<(), BlockchainError> {
    while b.get_height()? > 0 {
        assert_eq!(b.currency_in_circulation()?, Amount(2000000000000000000));
        b.rollback()?;
    }
    assert_eq!(b.currency_in_circulation()?, Amount(0));
    assert!(matches!(
        b.rollback(),
        Err(BlockchainError::NoBlocksToRollback)
    ));
    assert!(b
        .database
        .pairs("".into())?
        .into_iter()
        .collect::<Vec<_>>()
        .is_empty());
    Ok(())
}

#[test]
fn test_get_header_and_get_block() {
    let miner = TxBuilder::new(&Vec::from("VALIDATOR"));
    let mut chain = KvStoreChain::new(
        db::RamKvStore::new(),
        blockchain::get_test_blockchain_config(),
    )
    .unwrap();

    let new_block = chain.draft_block(60, &[], &miner, true).unwrap().unwrap();
    chain.extend(1, &[new_block.clone()]).unwrap();

    assert_eq!(chain.get_block(1).unwrap(), new_block);
    assert_eq!(chain.get_header(1).unwrap(), new_block.header);

    assert!(matches!(
        chain.get_block(2),
        Err(BlockchainError::BlockNotFound)
    ));
    assert!(matches!(
        chain.get_header(2),
        Err(BlockchainError::BlockNotFound)
    ));

    let mut broken_chain = chain.fork_on_ram();

    broken_chain
        .database
        .update(&vec![WriteOp::Put("HGT".into(), 3u64.into())])
        .unwrap();

    assert!(matches!(
        broken_chain.get_block(2),
        Err(BlockchainError::Inconsistency)
    ));
    assert!(matches!(
        broken_chain.get_header(2),
        Err(BlockchainError::Inconsistency)
    ));

    assert!(matches!(
        rollback_till_empty(&mut broken_chain),
        Err(BlockchainError::Inconsistency)
    ));
    rollback_till_empty(&mut chain).unwrap();
}

#[test]
fn test_timestamp_increasing() {
    let miner = TxBuilder::new(&Vec::from("VALIDATOR"));
    let mut chain = KvStoreChain::new(
        db::RamKvStore::new(),
        blockchain::get_test_blockchain_config(),
    )
    .unwrap();

    let mut fork1 = chain.fork_on_ram();
    fork1
        .apply_block(&fork1.draft_block(100, &[], &miner, true).unwrap().unwrap())
        .unwrap();
    assert!(matches!(
        fork1.draft_block(
            60, // 60 < 100
            &[],
            &miner,
            true,
        ),
        Err(BlockchainError::InvalidEpochSlot)
    ));
    assert!(matches!(
        &fork1.draft_block(
            101, // 101, same epoch!
            &[],
            &miner,
            true,
        ),
        Err(BlockchainError::InvalidEpochSlot)
    ));

    for i in 11..30 {
        fork1
            .apply_block(
                &fork1
                    .draft_block(i * 60, &[], &miner, true)
                    .unwrap()
                    .unwrap(),
            )
            .unwrap();
    }

    assert!(matches!(
        fork1.draft_block(28 * 60, &[], &miner, true,),
        Err(BlockchainError::InvalidEpochSlot)
    ));
    fork1
        .apply_block(
            &fork1
                .draft_block(31 * 60, &[], &miner, true)
                .unwrap()
                .unwrap(),
        )
        .unwrap();

    rollback_till_empty(&mut fork1).unwrap();
    drop(fork1);
    rollback_till_empty(&mut chain).unwrap();
}

#[test]
fn test_block_number_correctness_check() {
    let miner = TxBuilder::new(&Vec::from("VALIDATOR"));
    let mut chain = KvStoreChain::new(
        db::RamKvStore::new(),
        blockchain::get_test_blockchain_config(),
    )
    .unwrap();
    let mut fork1 = chain.fork_on_ram();
    let blk1 = fork1.draft_block(100, &[], &miner, true).unwrap().unwrap();
    fork1.extend(1, &[blk1.clone()]).unwrap();
    let blk2 = fork1.draft_block(200, &[], &miner, true).unwrap().unwrap();
    fork1.extend(2, &[blk2.clone()]).unwrap();
    assert_eq!(fork1.get_height().unwrap(), 3);

    let mut fork2 = chain.fork_on_ram();
    fork2.extend(1, &[blk1.clone(), blk2.clone()]).unwrap();
    assert_eq!(fork2.get_height().unwrap(), 3);

    let mut fork3 = chain.fork_on_ram();
    let mut blk1_wrong_num = blk1.clone();
    blk1_wrong_num.header.number += 1;
    assert!(matches!(
        fork3.extend(1, &[blk1_wrong_num, blk2.clone()]),
        Err(BlockchainError::InvalidBlockNumber)
    ));

    let mut fork4 = chain.fork_on_ram();
    let mut blk2_wrong_num = blk2.clone();
    blk2_wrong_num.header.number += 1;
    assert!(matches!(
        fork4.extend(1, &[blk1, blk2_wrong_num.clone()]),
        Err(BlockchainError::InvalidBlockNumber)
    ));

    rollback_till_empty(&mut fork1).unwrap();
    rollback_till_empty(&mut fork2).unwrap();
    rollback_till_empty(&mut fork3).unwrap();
    rollback_till_empty(&mut fork4).unwrap();
    drop((fork1, fork2, fork3, fork4));
    rollback_till_empty(&mut chain).unwrap();
}

#[test]
fn test_parent_hash_correctness_check() {
    let miner = TxBuilder::new(&Vec::from("VALIDATOR"));
    let mut chain = KvStoreChain::new(
        db::RamKvStore::new(),
        blockchain::get_test_blockchain_config(),
    )
    .unwrap();
    let mut fork1 = chain.fork_on_ram();
    let blk1 = fork1.draft_block(100, &[], &miner, true).unwrap().unwrap();
    fork1.extend(1, &[blk1.clone()]).unwrap();
    let blk2 = fork1.draft_block(200, &[], &miner, true).unwrap().unwrap();
    fork1.extend(2, &[blk2.clone()]).unwrap();
    assert_eq!(fork1.get_height().unwrap(), 3);

    let mut fork2 = chain.fork_on_ram();
    fork2.extend(1, &[blk1.clone(), blk2.clone()]).unwrap();
    assert_eq!(fork2.get_height().unwrap(), 3);

    let mut fork3 = chain.fork_on_ram();
    let mut blk1_wrong = blk1.clone();
    blk1_wrong.header.parent_hash = Default::default();
    assert!(matches!(
        fork3.extend(1, &[blk1_wrong, blk2.clone()]),
        Err(BlockchainError::InvalidParentHash)
    ));

    let mut fork4 = chain.fork_on_ram();
    let mut blk2_wrong = blk2.clone();
    blk2_wrong.header.parent_hash = Default::default();
    assert!(matches!(
        fork4.extend(1, &[blk1, blk2_wrong.clone()]),
        Err(BlockchainError::InvalidParentHash)
    ));

    rollback_till_empty(&mut fork1).unwrap();
    rollback_till_empty(&mut fork2).unwrap();
    rollback_till_empty(&mut fork3).unwrap();
    rollback_till_empty(&mut fork4).unwrap();
    drop((fork1, fork2, fork3, fork4));
    rollback_till_empty(&mut chain).unwrap();
}

#[test]
fn test_merkle_root_check() {
    let alice = TxBuilder::new(&Vec::from("ABC"));
    let miner = TxBuilder::new(&Vec::from("VALIDATOR"));
    let mut chain = KvStoreChain::new(
        db::RamKvStore::new(),
        blockchain::get_test_blockchain_config(),
    )
    .unwrap();
    let blk1 = chain
        .draft_block(
            100,
            &[
                alice.create_transaction(
                    "".into(),
                    miner.get_address(),
                    Money::ziesha(100),
                    Money::ziesha(0),
                    1,
                ),
                alice.create_transaction(
                    "".into(),
                    miner.get_address(),
                    Money::ziesha(200),
                    Money::ziesha(0),
                    2,
                ),
            ],
            &miner,
            true,
        )
        .unwrap()
        .unwrap();
    let blk2 = chain
        .draft_block(
            200,
            &[
                alice.create_transaction(
                    "".into(),
                    miner.get_address(),
                    Money::ziesha(200),
                    Money::ziesha(0),
                    1,
                ),
                alice.create_transaction(
                    "".into(),
                    miner.get_address(),
                    Money::ziesha(100),
                    Money::ziesha(0),
                    2,
                ),
            ],
            &miner,
            true,
        )
        .unwrap()
        .unwrap();

    let mut fork1 = chain.fork_on_ram();
    let mut fork2 = chain.fork_on_ram();

    fork1.apply_block(&blk1).unwrap();
    assert_eq!(
        fork1
            .get_balance(alice.get_address(), ContractId::Ziesha)
            .unwrap(),
        Amount(9700)
    );

    fork2.apply_block(&blk2).unwrap();
    assert_eq!(
        fork2
            .get_balance(alice.get_address(), ContractId::Ziesha)
            .unwrap(),
        Amount(9700)
    );

    let mut blk_wrong = blk1.clone();
    blk_wrong.header.block_root = Default::default();
    assert!(matches!(
        chain.fork_on_ram().apply_block(&blk_wrong),
        Err(BlockchainError::InvalidMerkleRoot)
    ));

    rollback_till_empty(&mut fork1).unwrap();
    rollback_till_empty(&mut fork2).unwrap();
    drop((fork1, fork2));
    rollback_till_empty(&mut chain).unwrap();
}

#[test]
fn test_txs_cant_be_duplicated() {
    let miner = TxBuilder::new(&Vec::from("VALIDATOR"));
    let alice = TxBuilder::new(&Vec::from("ABC"));
    let bob = TxBuilder::new(&Vec::from("CBA"));

    let mut chain = KvStoreChain::new(
        db::RamKvStore::new(),
        blockchain::get_test_blockchain_config(),
    )
    .unwrap();

    // Alice: 10000 Bob: 0
    assert_eq!(
        chain
            .get_balance(alice.get_address(), ContractId::Ziesha)
            .unwrap(),
        Amount(10000)
    );
    assert_eq!(
        chain
            .get_balance(bob.get_address(), ContractId::Ziesha)
            .unwrap(),
        Amount(0)
    );

    let tx = alice.create_transaction(
        "".into(),
        bob.get_address(),
        Money::ziesha(2700),
        Money::ziesha(300),
        1,
    );

    // Alice -> 2700 -> Bob (Fee 300)
    chain
        .apply_block(
            &chain
                .draft_block(100, &[tx.clone()], &miner, true)
                .unwrap()
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        chain
            .get_balance(alice.get_address(), ContractId::Ziesha)
            .unwrap(),
        Amount(7000)
    );
    assert_eq!(
        chain
            .get_balance(bob.get_address(), ContractId::Ziesha)
            .unwrap(),
        Amount(2700)
    );

    // Alice -> 2700 -> Bob (Fee 300) (NOT APPLIED: DUPLICATED TRANSACTION!)
    chain
        .apply_block(
            &chain
                .draft_block(200, &[tx.clone()], &miner, true)
                .unwrap()
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        chain
            .get_balance(alice.get_address(), ContractId::Ziesha)
            .unwrap(),
        Amount(7000)
    );
    assert_eq!(
        chain
            .get_balance(bob.get_address(), ContractId::Ziesha)
            .unwrap(),
        Amount(2700)
    );

    let tx2 = alice.create_transaction(
        "".into(),
        bob.get_address(),
        Money::ziesha(2700),
        Money::ziesha(300),
        2,
    );

    // Alice -> 2700 -> Bob (Fee 300)
    chain
        .apply_block(
            &chain
                .draft_block(300, &[tx2], &miner, true)
                .unwrap()
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        chain
            .get_balance(alice.get_address(), ContractId::Ziesha)
            .unwrap(),
        Amount(4000)
    );
    assert_eq!(
        chain
            .get_balance(bob.get_address(), ContractId::Ziesha)
            .unwrap(),
        Amount(5400)
    );

    rollback_till_empty(&mut chain).unwrap();
}

#[test]
fn test_insufficient_balance_is_handled() {
    let miner = TxBuilder::new(&Vec::from("VALIDATOR"));
    let alice = TxBuilder::new(&Vec::from("ABC"));
    let bob = TxBuilder::new(&Vec::from("CBA"));

    let mut chain = KvStoreChain::new(
        db::RamKvStore::new(),
        blockchain::get_test_blockchain_config(),
    )
    .unwrap();

    // Alice: 10000 Bob: 0
    assert_eq!(
        chain
            .get_balance(alice.get_address(), ContractId::Ziesha)
            .unwrap(),
        Amount(10000)
    );
    assert_eq!(
        chain
            .get_balance(bob.get_address(), ContractId::Ziesha)
            .unwrap(),
        Amount(0)
    );

    let tx = alice.create_transaction(
        "".into(),
        bob.get_address(),
        Money::ziesha(9701),
        Money::ziesha(300),
        1,
    );

    // Ensure apply_tx will raise
    match chain.apply_tx(&tx.tx, false) {
        Ok(_) => assert!(
            false,
            "Transaction from wallet with insufficient fund should fail"
        ),
        Err(e) => assert!(matches!(e, BlockchainError::BalanceInsufficient)),
    }

    // Ensure tx is not included in block and bob has not received funds
    chain
        .apply_block(
            &chain
                .draft_block(100, &[tx], &miner, true)
                .unwrap()
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        chain
            .get_balance(bob.get_address(), ContractId::Ziesha)
            .unwrap(),
        Amount(0)
    );

    rollback_till_empty(&mut chain).unwrap();
}

#[test]
fn test_cant_apply_unsigned_tx() {
    let miner = TxBuilder::new(&Vec::from("VALIDATOR"));
    let alice = TxBuilder::new(&Vec::from("ABC"));
    let bob = TxBuilder::new(&Vec::from("CBA"));

    let mut chain = KvStoreChain::new(
        db::RamKvStore::new(),
        blockchain::get_test_blockchain_config(),
    )
    .unwrap();

    // Create unsigned signed tx
    let unsigned_tx = Transaction {
        memo: "".into(),
        src: Some(alice.get_address()),
        data: TransactionData::RegularSend {
            entries: vec![RegularSendEntry {
                dst: bob.get_address(),
                amount: Money::ziesha(1000),
            }],
        },
        nonce: 1,
        fee: Money::ziesha(300),
        sig: Signature::Unsigned,
    };
    let unsigned_tx = TransactionAndDelta {
        tx: unsigned_tx,
        state_delta: None,
    };

    // Ensure apply_tx will raise
    match chain.draft_block(100, &[unsigned_tx.clone()], &miner, false) {
        Ok(_) => assert!(false, "Unsigned transaction shall not be applied"),
        Err(e) => assert!(matches!(e, BlockchainError::SignatureError)),
    }

    // Ensure tx is not included in block and bob has not received funds
    chain
        .apply_block(
            &chain
                .draft_block(200, &[unsigned_tx], &miner, true)
                .unwrap()
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        chain
            .get_balance(bob.get_address(), ContractId::Ziesha)
            .unwrap(),
        Amount(0)
    );

    rollback_till_empty(&mut chain).unwrap();
}

#[test]
fn test_cant_apply_invalid_signed_tx() {
    let miner = TxBuilder::new(&Vec::from("VALIDATOR"));
    let alice = TxBuilder::new(&Vec::from("ABC"));
    let bob = TxBuilder::new(&Vec::from("CBA"));

    let mut chain = KvStoreChain::new(
        db::RamKvStore::new(),
        blockchain::get_test_blockchain_config(),
    )
    .unwrap();

    // Create unsigned tx
    let (_, sk) = Signer::generate_keys(&Vec::from("ABC"));
    let mut tx = Transaction {
        memo: "".into(),
        src: Some(alice.get_address()),
        data: TransactionData::RegularSend {
            entries: vec![RegularSendEntry {
                dst: bob.get_address(),
                amount: Money::ziesha(1000),
            }],
        },
        nonce: 1,
        fee: Money::ziesha(300),
        sig: Signature::Unsigned,
    };

    let mut bytes = bincode::serialize(&tx).unwrap();
    bytes.push(0x11);

    tx.sig = Signature::Signed(Signer::sign(&sk, &bytes));
    let tx = TransactionAndDelta {
        tx,
        state_delta: None,
    };

    // Ensure apply_tx will raise
    match chain.draft_block(100, &[tx.clone()], &miner, false) {
        Ok(_) => assert!(false, "Invalid signed transaction shall not be applied"),
        Err(e) => assert!(matches!(e, BlockchainError::SignatureError)),
    }

    // Ensure tx is not included in block and bob has not received funds
    chain
        .apply_block(
            &chain
                .draft_block(100, &[tx], &miner, true)
                .unwrap()
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        chain
            .get_balance(bob.get_address(), ContractId::Ziesha)
            .unwrap(),
        Amount(0)
    );

    rollback_till_empty(&mut chain).unwrap();
}

#[test]
fn test_balances_are_correct_after_tx() {
    let miner = TxBuilder::new(&Vec::from("VALIDATOR"));
    let alice = TxBuilder::new(&Vec::from("ABC"));
    let bob = TxBuilder::new(&Vec::from("CBA"));

    let mut chain = KvStoreChain::new(
        db::RamKvStore::new(),
        blockchain::get_test_blockchain_config(),
    )
    .unwrap();

    // Alice: 10000 Bob: 0
    assert_eq!(
        chain
            .get_balance(alice.get_address(), ContractId::Ziesha)
            .unwrap(),
        Amount(10000)
    );
    assert_eq!(
        chain
            .get_balance(bob.get_address(), ContractId::Ziesha)
            .unwrap(),
        Amount(0)
    );

    // Alice -> 2700 -> Bob (Fee 300)
    chain
        .apply_block(
            &chain
                .draft_block(
                    100,
                    &[alice.create_transaction(
                        "".into(),
                        bob.get_address(),
                        Money::ziesha(2700),
                        Money::ziesha(300),
                        1,
                    )],
                    &miner,
                    true,
                )
                .unwrap()
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        chain
            .get_balance(alice.get_address(), ContractId::Ziesha)
            .unwrap(),
        Amount(7000)
    );
    assert_eq!(
        chain
            .get_balance(bob.get_address(), ContractId::Ziesha)
            .unwrap(),
        Amount(2700)
    );

    // Bob -> 2600 -> Alice (Fee 200) (BALANCE INSUFFICIENT!)
    chain
        .apply_block(
            &chain
                .draft_block(
                    200,
                    &[bob.create_transaction(
                        "".into(),
                        alice.get_address(),
                        Money::ziesha(2600),
                        Money::ziesha(200),
                        1,
                    )],
                    &miner,
                    true,
                )
                .unwrap()
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        chain
            .get_balance(alice.get_address(), ContractId::Ziesha)
            .unwrap(),
        Amount(7000)
    );
    assert_eq!(
        chain
            .get_balance(bob.get_address(), ContractId::Ziesha)
            .unwrap(),
        Amount(2700)
    );

    // Bob -> 2600 -> Alice (Fee 200)
    chain
        .apply_block(
            &chain
                .draft_block(
                    300,
                    &[bob.create_transaction(
                        "".into(),
                        alice.get_address(),
                        Money::ziesha(2600),
                        Money::ziesha(100),
                        1,
                    )],
                    &miner,
                    true,
                )
                .unwrap()
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        chain
            .get_balance(alice.get_address(), ContractId::Ziesha)
            .unwrap(),
        Amount(9600)
    );
    assert_eq!(
        chain
            .get_balance(bob.get_address(), ContractId::Ziesha)
            .unwrap(),
        Amount(0)
    );

    // Alice -> 100 -> Alice (Fee 200) (SELF PAYMENT NOT ALLOWED)
    chain
        .apply_block(
            &chain
                .draft_block(
                    400,
                    &[alice.create_transaction(
                        "".into(),
                        alice.get_address(),
                        Money::ziesha(100),
                        Money::ziesha(200),
                        2,
                    )],
                    &miner,
                    true,
                )
                .unwrap()
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        chain
            .get_balance(alice.get_address(), ContractId::Ziesha)
            .unwrap(),
        Amount(9400)
    );
    assert_eq!(
        chain
            .get_balance(bob.get_address(), ContractId::Ziesha)
            .unwrap(),
        Amount(0)
    );

    // Alice -> 20000 -> Alice (Fee 9400) (SELF PAYMENT NOT ALLOWED)
    chain
        .apply_block(
            &chain
                .draft_block(
                    500,
                    &[alice.create_transaction(
                        "".into(),
                        alice.get_address(),
                        Money::ziesha(20000),
                        Money::ziesha(9400),
                        3,
                    )],
                    &miner,
                    true,
                )
                .unwrap()
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        chain
            .get_balance(alice.get_address(), ContractId::Ziesha)
            .unwrap(),
        Amount(0)
    );
    assert_eq!(
        chain
            .get_balance(bob.get_address(), ContractId::Ziesha)
            .unwrap(),
        Amount(0)
    );

    rollback_till_empty(&mut chain).unwrap();
}

#[test]
fn test_genesis_is_not_replaceable() {
    let conf = blockchain::get_blockchain_config();
    let mut chain = KvStoreChain::new(db::RamKvStore::new(), conf.clone()).unwrap();
    assert_eq!(1, chain.get_height().unwrap());

    let first_block = chain.get_block(0).unwrap();
    assert_eq!(conf.genesis.header.hash(), first_block.header.hash());

    let mut another_conf = conf.clone();
    another_conf.genesis.header.proof_of_stake.timestamp += 1;

    assert!(matches!(
        chain.extend(0, &[another_conf.genesis]),
        Err(BlockchainError::ExtendFromGenesis)
    ));

    rollback_till_empty(&mut chain).unwrap();
}

#[test]
fn test_chain_should_not_draft_invalid_transactions() {
    let wallet_miner = TxBuilder::new(&Vec::from("VALIDATOR"));
    let wallet1 = TxBuilder::new(&Vec::from("ABC"));
    let wallet2 = TxBuilder::new(&Vec::from("CBA"));

    let mut conf = blockchain::get_test_blockchain_config();
    conf.genesis.body.push(Transaction {
        memo: "".into(),
        src: None,
        data: TransactionData::RegularSend {
            entries: vec![RegularSendEntry {
                dst: wallet1.get_address(),
                amount: Money::ziesha(10_000_000),
            }],
        },
        nonce: 0,
        fee: Money::ziesha(0),
        sig: Signature::Unsigned,
    });

    let mut chain = KvStoreChain::new(db::RamKvStore::new(), conf).unwrap();

    let t_valid = wallet1.create_transaction(
        "".into(),
        wallet2.get_address(),
        Money::ziesha(200),
        Money::ziesha(0),
        1,
    );
    let t_invalid_unsigned = TransactionAndDelta {
        tx: Transaction {
            memo: "".into(),
            src: Some(wallet1.get_address()),
            data: TransactionData::RegularSend {
                entries: vec![RegularSendEntry {
                    dst: wallet2.get_address(),
                    amount: Money::ziesha(300),
                }],
            },
            nonce: 1,
            fee: Money::ziesha(0),
            sig: Signature::Unsigned, // invalid transaction
        },
        state_delta: None,
    };
    let t_invalid_from_treasury = TransactionAndDelta {
        tx: Transaction {
            memo: "".into(),
            src: None,
            data: TransactionData::RegularSend {
                entries: vec![RegularSendEntry {
                    dst: wallet2.get_address(),
                    amount: Money::ziesha(500),
                }],
            },
            nonce: 1,
            fee: Money::ziesha(0),
            sig: Signature::Unsigned, // invalid transaction
        },
        state_delta: None,
    };
    let mempool = vec![t_valid, t_invalid_unsigned, t_invalid_from_treasury];
    let draft = chain
        .draft_block(1650000000, &mempool, &wallet_miner, true)
        .unwrap()
        .unwrap();

    assert_eq!(1, draft.body.len());
    assert_eq!(Some(wallet1.get_address()), draft.body[0].src);

    rollback_till_empty(&mut chain).unwrap();
}

#[test]
fn test_chain_should_draft_all_valid_transactions() {
    let wallet_miner = TxBuilder::new(&Vec::from("VALIDATOR"));
    let wallet1 = TxBuilder::new(&Vec::from("ABCD"));
    let wallet2 = TxBuilder::new(&Vec::from("CBAD"));

    let mut conf = blockchain::get_test_blockchain_config();
    conf.genesis.body.push(Transaction {
        memo: "".into(),
        src: None,
        data: TransactionData::RegularSend {
            entries: vec![RegularSendEntry {
                dst: wallet1.get_address(),
                amount: Money::ziesha(10_000_000),
            }],
        },
        nonce: 0,
        fee: Money::ziesha(0),
        sig: Signature::Unsigned,
    });

    let mut chain = KvStoreChain::new(db::RamKvStore::new(), conf).unwrap();

    let t1 = wallet1.create_transaction(
        "".into(),
        wallet2.get_address(),
        Money::ziesha(3000),
        Money::ziesha(0),
        1,
    );
    let t2 = wallet1.create_transaction(
        "".into(),
        wallet2.get_address(),
        Money::ziesha(4000),
        Money::ziesha(0),
        2,
    );

    let mempool = vec![t1, t2];
    let draft = chain
        .draft_block(1650000000, &mempool, &wallet_miner, true)
        .unwrap()
        .unwrap();

    chain.apply_block(&draft).unwrap();

    assert_eq!(2, draft.body.len());

    let account1 = chain
        .get_balance(wallet1.get_address(), ContractId::Ziesha)
        .unwrap();
    let account2 = chain
        .get_balance(wallet2.get_address(), ContractId::Ziesha)
        .unwrap();
    assert_eq!(Amount(10_000_000 - 7000), account1);
    assert_eq!(Amount(7000), account2);

    rollback_till_empty(&mut chain).unwrap();
}

#[test]
fn test_chain_should_rollback_applied_block() {
    let wallet_miner = TxBuilder::new(&Vec::from("VALIDATOR"));
    let wallet1 = TxBuilder::new(&Vec::from("ABC"));
    let wallet2 = TxBuilder::new(&Vec::from("CBA"));

    let mut conf = blockchain::get_test_blockchain_config();
    conf.genesis.body.push(Transaction {
        memo: "".into(),
        src: None,
        data: TransactionData::RegularSend {
            entries: vec![RegularSendEntry {
                dst: wallet1.get_address(),
                amount: Money::ziesha(10_000_000),
            }],
        },
        nonce: 0,
        fee: Money::ziesha(0),
        sig: Signature::Unsigned,
    });

    let mut chain = KvStoreChain::new(db::RamKvStore::new(), conf).unwrap();

    let t1 = wallet1.create_transaction(
        "".into(),
        wallet2.get_address(),
        Money::ziesha(1_000_000),
        Money::ziesha(0),
        1,
    );
    let mut mempool = vec![t1];
    let draft = chain
        .draft_block(1650000000, &mempool, &wallet_miner, true)
        .unwrap()
        .unwrap();

    chain.apply_block(&draft).unwrap();

    let t2 = wallet1.create_transaction(
        "".into(),
        wallet2.get_address(),
        Money::ziesha(500_000),
        Money::ziesha(0),
        2,
    );
    mempool.push(t2);

    let draft = chain
        .draft_block(1650000100, &mempool, &wallet_miner, true)
        .unwrap()
        .unwrap();

    let prev_checksum = chain
        .database
        .pairs("".into())
        .unwrap()
        .checksum::<Hasher>()
        .unwrap();

    chain.apply_block(&draft).unwrap();

    let height = chain.get_height().unwrap();
    assert_eq!(3, height);

    let last_block = chain.get_block(height - 1).unwrap();
    assert_eq!(
        last_block.body[0].data.clone(),
        TransactionData::RegularSend {
            entries: vec![RegularSendEntry {
                dst: wallet2.get_address(),
                amount: Money::ziesha(500_000)
            }]
        }
    );

    let after_checksum = chain
        .database
        .pairs("".into())
        .unwrap()
        .checksum::<Hasher>()
        .unwrap();

    chain.rollback().unwrap();

    let rollbacked_checksum = chain
        .database
        .pairs("".into())
        .unwrap()
        .checksum::<Hasher>()
        .unwrap();

    assert_ne!(prev_checksum, after_checksum);
    assert_eq!(prev_checksum, rollbacked_checksum);

    let height = chain.get_height().unwrap();
    assert_eq!(2, height);

    let nonce = chain.get_nonce(wallet2.get_address()).unwrap();
    let balance = chain
        .get_balance(wallet2.get_address(), ContractId::Ziesha)
        .unwrap();
    assert_eq!(Amount(1_000_000), balance);
    assert_eq!(0, nonce);

    rollback_till_empty(&mut chain).unwrap();
}
