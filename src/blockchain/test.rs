use super::*;
use crate::config::genesis;
use crate::core::{Address, Signature, TransactionData};
use crate::crypto::{EdDSA, SignatureScheme};
use crate::db;

fn easy_genesis() -> BlockAndPatch {
    let alice = Wallet::new(Vec::from("ABC"));
    let mut genesis_block = genesis::get_test_genesis_block();
    genesis_block.block.header.proof_of_work.target = 0x00ffffff;
    genesis_block.block.body = vec![Transaction {
        src: Address::Treasury,
        data: TransactionData::RegularSend {
            dst: alice.get_address(),
            amount: 10_000,
        },
        nonce: 1,
        fee: 0,
        sig: Signature::Unsigned,
    }];
    genesis_block
}

#[test]
fn test_block_number_correctness_check() -> Result<(), BlockchainError> {
    let miner = Wallet::new(Vec::from("MINER"));
    let chain = KvStoreChain::new(db::RamKvStore::new(), easy_genesis())?;
    let mut fork1 = chain.fork_on_ram();
    let blk1 = fork1.draft_block(0, &[], &miner)?;
    fork1.extend(1, &[blk1.block.clone()])?;
    let blk2 = fork1.draft_block(1, &[], &miner)?;
    fork1.extend(2, &[blk2.block.clone()])?;
    assert_eq!(fork1.get_height()?, 3);

    let mut fork2 = chain.fork_on_ram();
    fork2.extend(1, &[blk1.block.clone(), blk2.block.clone()])?;
    assert_eq!(fork2.get_height()?, 3);

    let mut fork3 = chain.fork_on_ram();
    let mut blk1_wrong_num = blk1.clone();
    blk1_wrong_num.block.header.number += 1;
    assert!(matches!(
        fork3.extend(1, &[blk1_wrong_num.block, blk2.block.clone()]),
        Err(BlockchainError::InvalidBlockNumber)
    ));

    let mut fork4 = chain.fork_on_ram();
    let mut blk2_wrong_num = blk2.clone();
    blk2_wrong_num.block.header.number += 1;
    assert!(matches!(
        fork4.extend(1, &[blk1.block, blk2_wrong_num.block.clone()]),
        Err(BlockchainError::InvalidBlockNumber)
    ));

    Ok(())
}

#[test]
fn test_parent_hash_correctness_check() -> Result<(), BlockchainError> {
    let miner = Wallet::new(Vec::from("MINER"));
    let chain = KvStoreChain::new(db::RamKvStore::new(), easy_genesis())?;
    let mut fork1 = chain.fork_on_ram();
    let blk1 = fork1.draft_block(0, &[], &miner)?;
    fork1.extend(1, &[blk1.block.clone()])?;
    let blk2 = fork1.draft_block(1, &[], &miner)?;
    fork1.extend(2, &[blk2.block.clone()])?;
    assert_eq!(fork1.get_height()?, 3);

    let mut fork2 = chain.fork_on_ram();
    fork2.extend(1, &[blk1.block.clone(), blk2.block.clone()])?;
    assert_eq!(fork2.get_height()?, 3);

    let mut fork3 = chain.fork_on_ram();
    let mut blk1_wrong = blk1.clone();
    blk1_wrong.block.header.parent_hash = Default::default();
    assert!(matches!(
        fork3.extend(1, &[blk1_wrong.block, blk2.block.clone()]),
        Err(BlockchainError::InvalidParentHash)
    ));

    let mut fork4 = chain.fork_on_ram();
    let mut blk2_wrong = blk2.clone();
    blk2_wrong.block.header.parent_hash = Default::default();
    assert!(matches!(
        fork4.extend(1, &[blk1.block, blk2_wrong.block.clone()]),
        Err(BlockchainError::InvalidParentHash)
    ));

    Ok(())
}

#[test]
fn test_merkle_root_check() -> Result<(), BlockchainError> {
    let alice = Wallet::new(Vec::from("ABC"));
    let miner = Wallet::new(Vec::from("MINER"));
    let chain = KvStoreChain::new(db::RamKvStore::new(), easy_genesis())?;
    let blk1 = chain
        .draft_block(
            1,
            &[
                alice.create_transaction(miner.get_address(), 100, 0, 1),
                alice.create_transaction(miner.get_address(), 200, 0, 2),
            ],
            &miner,
        )?
        .block;
    let blk2 = chain
        .draft_block(
            1,
            &[
                alice.create_transaction(miner.get_address(), 200, 0, 1),
                alice.create_transaction(miner.get_address(), 100, 0, 2),
            ],
            &miner,
        )?
        .block;

    let mut fork1 = chain.fork_on_ram();
    let mut fork2 = chain.fork_on_ram();

    fork1.apply_block(&blk1, false)?;
    assert_eq!(fork1.get_account(alice.get_address())?.balance, 9700);

    fork2.apply_block(&blk2, false)?;
    assert_eq!(fork2.get_account(alice.get_address())?.balance, 9700);

    let mut blk_wrong = blk1.clone();
    blk_wrong.header.block_root = Default::default();
    assert!(matches!(
        chain.fork_on_ram().apply_block(&blk_wrong, false),
        Err(BlockchainError::InvalidMerkleRoot)
    ));

    Ok(())
}

#[test]
fn test_contract_create_patch() -> Result<(), BlockchainError> {
    let miner = Wallet::new(Vec::from("MINER"));
    let alice = Wallet::new(Vec::from("ABC"));
    let mut chain = KvStoreChain::new(db::RamKvStore::new(), easy_genesis())?;

    let state_model = zk::ZkStateModel::new(1, 3);
    let full_state = zk::ZkState::default();

    let tx = alice.create_contract(
        zk::ZkContract {
            state_model,
            initial_state: full_state.compress(state_model),
            deposit_withdraw: zk::ZkVerifierKey::Groth16(vec![]),
            update: Vec::new(),
        },
        full_state.clone(),
        0,
        1,
    );

    let draft = chain.draft_block(1, &[tx.clone()], &miner)?;
    chain.apply_block(&draft.block, false)?;

    assert_eq!(chain.get_height()?, 2);
    assert_eq!(chain.get_state_height()?, 1);

    chain.update_states(&draft.patch)?;

    assert_eq!(chain.get_height()?, 2);
    assert_eq!(chain.get_state_height()?, 2);

    Ok(())
}

#[test]
fn test_txs_cant_be_duplicated() -> Result<(), BlockchainError> {
    let miner = Wallet::new(Vec::from("MINER"));
    let alice = Wallet::new(Vec::from("ABC"));
    let bob = Wallet::new(Vec::from("CBA"));

    let mut chain = KvStoreChain::new(db::RamKvStore::new(), easy_genesis())?;

    // Alice: 10000 Bob: 0
    assert_eq!(chain.get_account(alice.get_address())?.balance, 10000);
    assert_eq!(chain.get_account(bob.get_address())?.balance, 0);

    let tx = alice.create_transaction(bob.get_address(), 2700, 300, 1);

    // Alice -> 2700 -> Bob (Fee 300)
    chain.apply_block(&chain.draft_block(1, &[tx.clone()], &miner)?.block, false)?;
    assert_eq!(chain.get_account(alice.get_address())?.balance, 7000);
    assert_eq!(chain.get_account(bob.get_address())?.balance, 2700);

    // Alice -> 2700 -> Bob (Fee 300) (NOT APPLIED: DUPLICATED TRANSACTION!)
    chain.apply_block(&chain.draft_block(1, &[tx.clone()], &miner)?.block, false)?;
    assert_eq!(chain.get_account(alice.get_address())?.balance, 7000);
    assert_eq!(chain.get_account(bob.get_address())?.balance, 2700);

    let tx2 = alice.create_transaction(bob.get_address(), 2700, 300, 2);

    // Alice -> 2700 -> Bob (Fee 300)
    chain.apply_block(&chain.draft_block(1, &[tx2], &miner)?.block, false)?;
    assert_eq!(chain.get_account(alice.get_address())?.balance, 4000);
    assert_eq!(chain.get_account(bob.get_address())?.balance, 5400);

    Ok(())
}

#[test]
fn test_insufficient_balance_is_handled() -> Result<(), BlockchainError> {
    let miner = Wallet::new(Vec::from("MINER"));
    let alice = Wallet::new(Vec::from("ABC"));
    let bob = Wallet::new(Vec::from("CBA"));

    let mut chain = KvStoreChain::new(db::RamKvStore::new(), easy_genesis())?;

    // Alice: 10000 Bob: 0
    assert_eq!(chain.get_account(alice.get_address())?.balance, 10000);
    assert_eq!(chain.get_account(bob.get_address())?.balance, 0);

    let tx = alice.create_transaction(bob.get_address(), 9701, 300, 1);

    // Ensure apply_tx will raise
    match chain.apply_tx(&tx.tx, false) {
        Ok(_) => assert!(
            false,
            "Transaction from wallet with insufficient fund should fail"
        ),
        Err(e) => assert!(matches!(e, BlockchainError::BalanceInsufficient)),
    }

    // Ensure tx is not included in block and bob has not received funds
    chain.apply_block(&chain.draft_block(1, &[tx], &miner)?.block, false)?;
    assert_eq!(chain.get_account(bob.get_address())?.balance, 0);

    Ok(())
}

#[test]
fn test_cant_apply_unsigned_tx() -> Result<(), BlockchainError> {
    let miner = Wallet::new(Vec::from("MINER"));
    let alice = Wallet::new(Vec::from("ABC"));
    let bob = Wallet::new(Vec::from("CBA"));

    let mut chain = KvStoreChain::new(db::RamKvStore::new(), easy_genesis())?;

    // Create unsigned signed tx
    let unsigned_tx = Transaction {
        src: alice.get_address(),
        data: TransactionData::RegularSend {
            dst: bob.get_address(),
            amount: 1000,
        },
        nonce: 1,
        fee: 300,
        sig: Signature::Unsigned,
    };
    let unsigned_tx = TransactionAndDelta {
        tx: unsigned_tx,
        state_delta: None,
    };

    // Ensure apply_tx will raise
    match chain.apply_tx(&unsigned_tx.tx, false) {
        Ok(_) => assert!(false, "Unsigned transaction shall not be applied"),
        Err(e) => assert!(matches!(e, BlockchainError::SignatureError)),
    }

    // Ensure tx is not included in block and bob has not received funds
    chain.apply_block(&chain.draft_block(1, &[unsigned_tx], &miner)?.block, false)?;
    assert_eq!(chain.get_account(bob.get_address())?.balance, 0);

    Ok(())
}

#[test]
fn test_cant_apply_invalid_signed_tx() -> Result<(), BlockchainError> {
    let miner = Wallet::new(Vec::from("MINER"));
    let alice = Wallet::new(Vec::from("ABC"));
    let bob = Wallet::new(Vec::from("CBA"));

    let mut chain = KvStoreChain::new(db::RamKvStore::new(), easy_genesis())?;

    // Create unsigned tx
    let (_, sk) = EdDSA::generate_keys(&Vec::from("ABC"));
    let mut tx = Transaction {
        src: alice.get_address(),
        data: TransactionData::RegularSend {
            dst: bob.get_address(),
            amount: 1000,
        },
        nonce: 1,
        fee: 300,
        sig: Signature::Unsigned,
    };

    let mut bytes = bincode::serialize(&tx).unwrap();
    bytes.push(0x11);

    tx.sig = Signature::Signed(EdDSA::sign(&sk, &bytes));
    let tx = TransactionAndDelta {
        tx,
        state_delta: None,
    };

    // Ensure apply_tx will raise
    match chain.apply_tx(&tx.tx, false) {
        Ok(_) => assert!(false, "Unsigned transaction shall not be applied"),
        Err(e) => assert!(matches!(e, BlockchainError::SignatureError)),
    }

    // Ensure tx is not included in block and bob has not received funds
    chain.apply_block(&chain.draft_block(1, &[tx], &miner)?.block, false)?;
    assert_eq!(chain.get_account(bob.get_address())?.balance, 0);

    Ok(())
}

#[test]
fn test_balances_are_correct_after_tx() -> Result<(), BlockchainError> {
    let miner = Wallet::new(Vec::from("MINER"));
    let alice = Wallet::new(Vec::from("ABC"));
    let bob = Wallet::new(Vec::from("CBA"));

    let mut chain = KvStoreChain::new(db::RamKvStore::new(), easy_genesis())?;

    // Alice: 10000 Bob: 0
    assert_eq!(chain.get_account(alice.get_address())?.balance, 10000);
    assert_eq!(chain.get_account(bob.get_address())?.balance, 0);

    // Alice -> 2700 -> Bob (Fee 300)
    chain.apply_block(
        &chain
            .draft_block(
                1,
                &[alice.create_transaction(bob.get_address(), 2700, 300, 1)],
                &miner,
            )?
            .block,
        false,
    )?;
    assert_eq!(chain.get_account(alice.get_address())?.balance, 7000);
    assert_eq!(chain.get_account(bob.get_address())?.balance, 2700);

    // Bob -> 2600 -> Alice (Fee 200) (BALANCE INSUFFICIENT!)
    chain.apply_block(
        &chain
            .draft_block(
                1,
                &[bob.create_transaction(alice.get_address(), 2600, 200, 1)],
                &miner,
            )?
            .block,
        false,
    )?;
    assert_eq!(chain.get_account(alice.get_address())?.balance, 7000);
    assert_eq!(chain.get_account(bob.get_address())?.balance, 2700);

    // Bob -> 2600 -> Alice (Fee 200)
    chain.apply_block(
        &chain
            .draft_block(
                2,
                &[bob.create_transaction(alice.get_address(), 2600, 100, 1)],
                &miner,
            )?
            .block,
        false,
    )?;
    assert_eq!(chain.get_account(alice.get_address())?.balance, 9600);
    assert_eq!(chain.get_account(bob.get_address())?.balance, 0);

    // Alice -> 100 -> Alice (Fee 200)
    chain.apply_block(
        &chain
            .draft_block(
                3,
                &[alice.create_transaction(alice.get_address(), 100, 200, 2)],
                &miner,
            )?
            .block,
        false,
    )?;
    assert_eq!(chain.get_account(alice.get_address())?.balance, 9400);
    assert_eq!(chain.get_account(bob.get_address())?.balance, 0);

    // Alice -> 20000 -> Alice (Fee 9400) (BALANCE INSUFFICIENT even though sending to herself)
    chain.apply_block(
        &chain
            .draft_block(
                4,
                &[alice.create_transaction(alice.get_address(), 20000, 9400, 3)],
                &miner,
            )?
            .block,
        false,
    )?;
    assert_eq!(chain.get_account(alice.get_address())?.balance, 9400);
    assert_eq!(chain.get_account(bob.get_address())?.balance, 0);

    // Alice -> 1000 -> Alice (Fee 8400)
    chain.apply_block(
        &chain
            .draft_block(
                5,
                &[alice.create_transaction(alice.get_address(), 1000, 8400, 3)],
                &miner,
            )?
            .block,
        false,
    )?;
    assert_eq!(chain.get_account(alice.get_address())?.balance, 1000);
    assert_eq!(chain.get_account(bob.get_address())?.balance, 0);

    Ok(())
}

#[test]
fn test_genesis_is_not_replaceable() -> Result<(), BlockchainError> {
    let genesis_block = genesis::get_genesis_block();
    let mut chain = KvStoreChain::new(db::RamKvStore::new(), genesis_block.clone())?;
    assert_eq!(1, chain.get_height()?);

    let first_block = chain.get_block(0)?;
    assert_eq!(genesis_block.block.header.hash(), first_block.header.hash());

    let mut another_genesis_block = genesis_block.clone();
    another_genesis_block.block.header.proof_of_work.timestamp += 1;

    assert!(matches!(
        chain.extend(0, &[another_genesis_block.block]),
        Err(BlockchainError::ExtendFromGenesis)
    ));

    Ok(())
}

#[test]
fn test_chain_should_apply_mined_draft_block() -> Result<(), BlockchainError> {
    let wallet_miner = Wallet::new(Vec::from("MINER"));
    let wallet1 = Wallet::new(Vec::from("ABC"));
    let wallet2 = Wallet::new(Vec::from("CBA"));

    let mut genesis_block = genesis::get_genesis_block();
    genesis_block.block.header.proof_of_work.target = 0x0000ffff;
    genesis_block.block.body = vec![Transaction {
        src: Address::Treasury,
        data: TransactionData::RegularSend {
            dst: wallet1.get_address(),
            amount: 10_000_000,
        },
        nonce: 1,
        fee: 0,
        sig: Signature::Unsigned,
    }];

    let mut chain = KvStoreChain::new(db::RamKvStore::new(), genesis_block)?;

    let t1 = wallet1.create_transaction(wallet2.get_address(), 100, 0, 1);
    let mempool = vec![t1];
    let mut draft = chain.draft_block(1650000000, &mempool, &wallet_miner)?;

    mine_block(&chain, &mut draft)?;
    chain.apply_block(&draft.block, false)?;

    let height = chain.get_height()?;
    assert_eq!(2, height);

    let last_block = chain.get_block(height - 1)?;
    let w2_address = wallet2.get_address();
    assert_eq!(
        last_block.body[1].data.clone(),
        TransactionData::RegularSend {
            dst: w2_address,
            amount: 100
        }
    );

    let account = chain.get_account(wallet2.get_address())?;
    assert_eq!(100, account.balance);
    assert_eq!(0, account.nonce);

    Ok(())
}

#[test]
fn test_chain_should_not_draft_invalid_transactions() -> Result<(), BlockchainError> {
    let wallet_miner = Wallet::new(Vec::from("MINER"));
    let wallet1 = Wallet::new(Vec::from("ABC"));
    let wallet2 = Wallet::new(Vec::from("CBA"));

    let mut genesis_block = genesis::get_test_genesis_block();
    genesis_block.block.body = vec![Transaction {
        src: Address::Treasury,
        data: TransactionData::RegularSend {
            dst: wallet1.get_address(),
            amount: 10_000_000,
        },
        nonce: 1,
        fee: 0,
        sig: Signature::Unsigned,
    }];

    let chain = KvStoreChain::new(db::RamKvStore::new(), genesis_block)?;

    let t_valid = wallet1.create_transaction(wallet2.get_address(), 200, 0, 1);
    let t_invalid_unsigned = TransactionAndDelta {
        tx: Transaction {
            src: wallet1.get_address(),
            data: TransactionData::RegularSend {
                dst: wallet2.get_address(),
                amount: 300,
            },
            nonce: 1,
            fee: 0,
            sig: Signature::Unsigned, // invalid transaction
        },
        state_delta: None,
    };
    let t_invalid_from_treasury = TransactionAndDelta {
        tx: Transaction {
            src: Address::Treasury,
            data: TransactionData::RegularSend {
                dst: wallet2.get_address(),
                amount: 500,
            },
            nonce: 1,
            fee: 0,
            sig: Signature::Unsigned, // invalid transaction
        },
        state_delta: None,
    };
    let mempool = vec![t_valid, t_invalid_unsigned, t_invalid_from_treasury];
    let mut draft = chain.draft_block(1650000000, &mempool, &wallet_miner)?;

    mine_block(&chain, &mut draft)?;

    assert_eq!(2, draft.block.body.len());
    assert_eq!(wallet1.get_address(), draft.block.body[1].src);

    Ok(())
}

#[test]
fn test_chain_should_draft_all_valid_transactions() -> Result<(), BlockchainError> {
    let wallet_miner = Wallet::new(Vec::from("MINER"));
    let wallet1 = Wallet::new(Vec::from("ABC"));
    let wallet2 = Wallet::new(Vec::from("CBA"));

    let mut genesis_block = genesis::get_test_genesis_block();
    genesis_block.block.body = vec![Transaction {
        src: Address::Treasury,
        data: TransactionData::RegularSend {
            dst: wallet1.get_address(),
            amount: 10_000_000,
        },
        nonce: 1,
        fee: 0,
        sig: Signature::Unsigned,
    }];

    let mut chain = KvStoreChain::new(db::RamKvStore::new(), genesis_block)?;

    let t1 = wallet1.create_transaction(wallet2.get_address(), 3000, 0, 1);
    let t2 = wallet1.create_transaction(wallet2.get_address(), 4000, 0, 2);

    let mempool = vec![t1, t2];
    let mut draft = chain.draft_block(1650000000, &mempool, &wallet_miner)?;

    mine_block(&chain, &mut draft)?;

    chain.apply_block(&draft.block, false)?;

    assert_eq!(3, draft.block.body.len());

    let account1 = chain.get_account(wallet1.get_address())?;
    let account2 = chain.get_account(wallet2.get_address())?;
    assert_eq!(10_000_000 - 7000, account1.balance);
    assert_eq!(7000, account2.balance);

    Ok(())
}

#[test]
fn test_chain_should_rollback_applied_block() -> Result<(), BlockchainError> {
    let wallet_miner = Wallet::new(Vec::from("MINER"));
    let wallet1 = Wallet::new(Vec::from("ABC"));
    let wallet2 = Wallet::new(Vec::from("CBA"));

    let mut genesis_block = genesis::get_test_genesis_block();
    genesis_block.block.body = vec![Transaction {
        src: Address::Treasury,
        data: TransactionData::RegularSend {
            dst: wallet1.get_address(),
            amount: 10_000_000,
        },
        nonce: 1,
        fee: 0,
        sig: Signature::Unsigned,
    }];

    let mut chain = KvStoreChain::new(db::RamKvStore::new(), genesis_block)?;

    let t1 = wallet1.create_transaction(wallet2.get_address(), 1_000_000, 0, 1);
    let mut mempool = vec![t1];
    let mut draft = chain.draft_block(1650000000, &mempool, &wallet_miner)?;

    mine_block(&chain, &mut draft)?;

    chain.apply_block(&draft.block, false)?;

    let t2 = wallet1.create_transaction(wallet2.get_address(), 500_000, 0, 2);
    mempool.push(t2);

    let mut draft = chain.draft_block(1650000001, &mempool, &wallet_miner)?;

    mine_block(&chain, &mut draft)?;

    chain.apply_block(&draft.block, false)?;

    let height = chain.get_height()?;
    assert_eq!(3, height);

    let last_block = chain.get_block(height - 1)?;
    assert_eq!(
        last_block.body[1].data.clone(),
        TransactionData::RegularSend {
            dst: wallet2.get_address(),
            amount: 500_000
        }
    );

    chain.rollback_block()?;

    let height = chain.get_height()?;
    assert_eq!(2, height);

    let account = chain.get_account(wallet2.get_address())?;
    assert_eq!(1_000_000, account.balance);
    assert_eq!(0, account.nonce);

    Ok(())
}

fn mine_block<B: Blockchain>(chain: &B, draft: &mut BlockAndPatch) -> Result<(), BlockchainError> {
    let pow_key = chain.pow_key(draft.block.header.number)?;

    if draft.block.header.meets_target(pow_key.as_slice()) {
        return Ok(());
    }

    draft.block.header.proof_of_work.nonce = 0;
    while !draft.block.header.meets_target(pow_key.as_slice()) {
        draft.block.header.proof_of_work.nonce += 1;
    }

    Ok(())
}
