use super::*;
use crate::config::genesis;
use crate::core::{Address, TransactionData};
use crate::db;

const DEFAULT_DIFFICULTY: u32 = 0x0000ffff;

#[test]
fn test_txs_cant_be_duplicated() -> Result<(), BlockchainError> {
    let miner = Wallet::new(Vec::from("MINER"));
    let alice = Wallet::new(Vec::from("ABC"));
    let bob = Wallet::new(Vec::from("CBA"));
    let mut genesis_block = genesis::get_test_genesis_block();
    genesis_block.header.proof_of_work.target = 0x00ffffff;
    genesis_block.body = vec![Transaction {
        src: Address::Treasury,
        data: TransactionData::RegularSend {
            dst: alice.get_address(),
            amount: 10_000,
        },
        nonce: 1,
        fee: 0,
        sig: Signature::Unsigned,
    }];

    let mut chain = KvStoreChain::new(db::RamKvStore::new(), genesis_block.clone())?;

    // Alice: 10000 Bob: 0
    assert_eq!(chain.get_account(alice.get_address())?.balance, 10000);
    assert_eq!(chain.get_account(bob.get_address())?.balance, 0);

    let tx = alice.create_transaction(bob.get_address(), 2700, 300, 1);

    // Alice -> 2700 -> Bob (Fee 300)
    chain.apply_block(&chain.draft_block(1, &[tx.clone()], &miner)?, false)?;
    assert_eq!(chain.get_account(alice.get_address())?.balance, 7000);
    assert_eq!(chain.get_account(bob.get_address())?.balance, 2700);

    // Alice -> 2700 -> Bob (Fee 300) (NOT APPLIED: DUPLICATED TRANSACTION!)
    chain.apply_block(&chain.draft_block(1, &[tx.clone()], &miner)?, false)?;
    assert_eq!(chain.get_account(alice.get_address())?.balance, 7000);
    assert_eq!(chain.get_account(bob.get_address())?.balance, 2700);

    let tx2 = alice.create_transaction(bob.get_address(), 2700, 300, 2);

    // Alice -> 2700 -> Bob (Fee 300)
    chain.apply_block(&chain.draft_block(1, &[tx2], &miner)?, false)?;
    assert_eq!(chain.get_account(alice.get_address())?.balance, 4000);
    assert_eq!(chain.get_account(bob.get_address())?.balance, 5400);

    Ok(())
}

#[test]
fn test_balances_are_correct_after_tx() -> Result<(), BlockchainError> {
    let miner = Wallet::new(Vec::from("MINER"));
    let alice = Wallet::new(Vec::from("ABC"));
    let bob = Wallet::new(Vec::from("CBA"));
    let mut genesis_block = genesis::get_test_genesis_block();
    genesis_block.header.proof_of_work.target = 0x00ffffff;
    genesis_block.body = vec![Transaction {
        src: Address::Treasury,
        data: TransactionData::RegularSend {
            dst: alice.get_address(),
            amount: 10_000,
        },
        nonce: 1,
        fee: 0,
        sig: Signature::Unsigned,
    }];

    let mut chain = KvStoreChain::new(db::RamKvStore::new(), genesis_block.clone())?;

    // Alice: 10000 Bob: 0
    assert_eq!(chain.get_account(alice.get_address())?.balance, 10000);
    assert_eq!(chain.get_account(bob.get_address())?.balance, 0);

    // Alice -> 2700 -> Bob (Fee 300)
    chain.apply_block(
        &chain.draft_block(
            1,
            &[alice.create_transaction(bob.get_address(), 2700, 300, 1)],
            &miner,
        )?,
        false,
    )?;
    assert_eq!(chain.get_account(alice.get_address())?.balance, 7000);
    assert_eq!(chain.get_account(bob.get_address())?.balance, 2700);

    // Bob -> 2600 -> Alice (Fee 200) (BALANCE INSUFFICIENT!)
    chain.apply_block(
        &chain.draft_block(
            1,
            &[bob.create_transaction(alice.get_address(), 2600, 200, 1)],
            &miner,
        )?,
        false,
    )?;
    assert_eq!(chain.get_account(alice.get_address())?.balance, 7000);
    assert_eq!(chain.get_account(bob.get_address())?.balance, 2700);

    // Bob -> 2600 -> Alice (Fee 200)
    chain.apply_block(
        &chain.draft_block(
            2,
            &[bob.create_transaction(alice.get_address(), 2600, 100, 1)],
            &miner,
        )?,
        false,
    )?;
    assert_eq!(chain.get_account(alice.get_address())?.balance, 9600);
    assert_eq!(chain.get_account(bob.get_address())?.balance, 0);

    // Alice -> 100 -> Alice (Fee 200)
    chain.apply_block(
        &chain.draft_block(
            3,
            &[alice.create_transaction(alice.get_address(), 100, 200, 2)],
            &miner,
        )?,
        false,
    )?;
    assert_eq!(chain.get_account(alice.get_address())?.balance, 9400);
    assert_eq!(chain.get_account(bob.get_address())?.balance, 0);

    // Alice -> 20000 -> Alice (Fee 9400) (BALANCE INSUFFICIENT even though sending to herself)
    chain.apply_block(
        &chain.draft_block(
            4,
            &[alice.create_transaction(alice.get_address(), 20000, 9400, 3)],
            &miner,
        )?,
        false,
    )?;
    assert_eq!(chain.get_account(alice.get_address())?.balance, 9400);
    assert_eq!(chain.get_account(bob.get_address())?.balance, 0);

    // Alice -> 1000 -> Alice (Fee 8400)
    chain.apply_block(
        &chain.draft_block(
            5,
            &[alice.create_transaction(alice.get_address(), 1000, 8400, 3)],
            &miner,
        )?,
        false,
    )?;
    assert_eq!(chain.get_account(alice.get_address())?.balance, 1000);
    assert_eq!(chain.get_account(bob.get_address())?.balance, 0);

    Ok(())
}

#[test]
fn test_empty_chain_should_have_genesis_block() -> Result<(), BlockchainError> {
    let genesis_block = genesis::get_genesis_block();
    let chain = KvStoreChain::new(db::RamKvStore::new(), genesis_block.clone())?;
    assert_eq!(1, chain.get_height()?);

    let first_block = chain.get_block(0)?;
    assert_eq!(genesis_block.header.hash(), first_block.header.hash());

    Ok(())
}

#[test]
fn test_chain_should_apply_mined_draft_block() -> Result<(), BlockchainError> {
    let wallet_miner = Wallet::new(Vec::from("MINER"));
    let wallet1 = Wallet::new(Vec::from("ABC"));
    let wallet2 = Wallet::new(Vec::from("CBA"));

    let mut genesis_block = genesis::get_genesis_block();
    genesis_block.header.proof_of_work.target = DEFAULT_DIFFICULTY;
    genesis_block.body = vec![Transaction {
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
    chain.apply_block(&draft, false)?;

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

    let mut genesis_block = genesis::get_genesis_block();
    genesis_block.header.proof_of_work.target = DEFAULT_DIFFICULTY;
    genesis_block.body = vec![Transaction {
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
    let t_invalid_unsigned = Transaction {
        src: wallet1.get_address(),
        data: TransactionData::RegularSend {
            dst: wallet2.get_address(),
            amount: 300,
        },
        nonce: 1,
        fee: 0,
        sig: Signature::Unsigned, // invalid transaction
    };
    let t_invalid_from_treasury = Transaction {
        src: Address::Treasury,
        data: TransactionData::RegularSend {
            dst: wallet2.get_address(),
            amount: 500,
        },
        nonce: 1,
        fee: 0,
        sig: Signature::Unsigned, // invalid transaction
    };
    let mempool = vec![t_valid, t_invalid_unsigned, t_invalid_from_treasury];
    let draft = chain.draft_block(1650000000, &mempool, &wallet_miner)?;

    assert_eq!(2, draft.body.len());
    assert_eq!(wallet1.get_address(), draft.body[1].src);

    Ok(())
}

#[test]
fn test_chain_should_draft_all_valid_transactions() -> Result<(), BlockchainError> {
    let wallet_miner = Wallet::new(Vec::from("MINER"));
    let wallet1 = Wallet::new(Vec::from("ABC"));
    let wallet2 = Wallet::new(Vec::from("CBA"));

    let mut genesis_block = genesis::get_genesis_block();
    genesis_block.header.proof_of_work.target = DEFAULT_DIFFICULTY;
    genesis_block.body = vec![Transaction {
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
    chain.apply_block(&draft, false)?;

    assert_eq!(3, draft.body.len());

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

    let mut genesis_block = genesis::get_genesis_block();
    genesis_block.header.proof_of_work.target = DEFAULT_DIFFICULTY;
    genesis_block.body = vec![Transaction {
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
    chain.apply_block(&draft, false)?;

    let t2 = wallet1.create_transaction(wallet2.get_address(), 500_000, 0, 2);
    mempool.push(t2);

    let mut draft = chain.draft_block(1650000001, &mempool, &wallet_miner)?;

    mine_block(&chain, &mut draft)?;
    chain.apply_block(&draft, false)?;

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

fn mine_block<B: Blockchain>(chain: &B, draft: &mut Block) -> Result<(), BlockchainError> {
    let pow_key = chain.pow_key(draft.header.number)?;

    if draft.header.meets_target(pow_key.as_slice()) {
        return Ok(());
    }

    draft.header.proof_of_work.nonce = 0;
    while !draft.header.meets_target(pow_key.as_slice()) {
        draft.header.proof_of_work.nonce += 1;
    }

    Ok(())
}
