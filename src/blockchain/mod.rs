use thiserror::Error;

use crate::config;
use crate::config::TOTAL_SUPPLY;
use crate::core::{
    Account, Address, Block, Header, Money, ProofOfWork, Signature, Transaction, TransactionData,
};
use crate::db::{KvStore, KvStoreError, RamMirrorKvStore, StringKey, WriteOp};
use crate::utils;
use crate::wallet::Wallet;
use crate::zk::ZkState;

#[derive(Error, Debug)]
pub enum BlockchainError {
    #[error("kvstore error happened: {0}")]
    KvStoreError(#[from] KvStoreError),
    #[error("transaction signature is invalid")]
    SignatureError,
    #[error("balance insufficient")]
    BalanceInsufficient,
    #[error("inconsistency error")]
    Inconsistency,
    #[error("block not found")]
    BlockNotFound,
    #[error("cannot extend from the genesis block")]
    ExtendFromGenesis,
    #[error("cannot extend from very future blocks")]
    ExtendFromFuture,
    #[error("block number invalid")]
    InvalidBlockNumber,
    #[error("parent hash invalid")]
    InvalidParentHash,
    #[error("merkle root invalid")]
    InvalidMerkleRoot,
    #[error("transaction nonce invalid")]
    InvalidTransactionNonce,
    #[error("block timestamp is in past")]
    InvalidTimestamp,
    #[error("unmet difficulty target")]
    DifficultyTargetUnmet,
    #[error("miner reward not present")]
    MinerRewardNotFound,
    #[error("illegal access to treasury funds")]
    IllegalTreasuryAccess,
    #[error("miner reward transaction is invalid")]
    InvalidMinerReward,
}

pub trait Blockchain {
    fn get_account(&self, addr: Address) -> Result<Account, BlockchainError>;
    fn next_reward(&self) -> Result<Money, BlockchainError>;
    fn will_extend(&self, from: usize, headers: &[Header]) -> Result<bool, BlockchainError>;
    fn extend(&mut self, from: usize, blocks: &[Block]) -> Result<(), BlockchainError>;
    fn draft_block(
        &self,
        timestamp: u32,
        mempool: &[Transaction],
        wallet: &Wallet,
    ) -> Result<Block, BlockchainError>;
    fn get_height(&self) -> Result<usize, BlockchainError>;
    fn get_headers(
        &self,
        since: usize,
        until: Option<usize>,
    ) -> Result<Vec<Header>, BlockchainError>;
    fn get_blocks(&self, since: usize, until: Option<usize>)
        -> Result<Vec<Block>, BlockchainError>;

    fn get_power(&self) -> Result<u64, BlockchainError>;

    fn pow_key(&self, index: usize) -> Result<Vec<u8>, BlockchainError>;
}

pub struct KvStoreChain<K: KvStore> {
    database: K,
}

impl<K: KvStore> KvStoreChain<K> {
    pub fn new(kv_store: K, genesis_block: Block) -> Result<KvStoreChain<K>, BlockchainError> {
        let mut chain = KvStoreChain::<K> { database: kv_store };
        if chain.get_height()? == 0 {
            chain.apply_block(&genesis_block, false)?;
        }
        Ok(chain)
    }

    fn fork_on_ram(&self) -> KvStoreChain<RamMirrorKvStore<'_, K>> {
        KvStoreChain {
            database: RamMirrorKvStore::new(&self.database),
        }
    }

    fn median_timestamp(&self, index: usize) -> Result<u32, BlockchainError> {
        Ok(utils::median(
            &(0..std::cmp::min(index + 1, config::MEDIAN_TIMESTAMP_COUNT))
                .map(|i| {
                    self.get_block(index - i)
                        .map(|b| b.header.proof_of_work.timestamp)
                })
                .collect::<Result<Vec<u32>, BlockchainError>>()?,
        ))
    }

    fn next_difficulty(&self) -> Result<u32, BlockchainError> {
        let height = self.get_height()?;
        let last_block = self.get_block(height - 1)?.header;
        if height % config::DIFFICULTY_CALC_INTERVAL == 0 {
            let prev_block = self
                .get_block(height - config::DIFFICULTY_CALC_INTERVAL)?
                .header;
            let time_delta =
                last_block.proof_of_work.timestamp - prev_block.proof_of_work.timestamp;
            let avg_block_time = time_delta / (config::DIFFICULTY_CALC_INTERVAL - 1) as u32;
            let diff_change =
                (config::BLOCK_TIME as f32 / avg_block_time as f32).clamp(0.5f32, 2f32);
            let new_diff =
                rust_randomx::Difficulty::new(last_block.proof_of_work.target).scale(diff_change);
            Ok(new_diff.to_u32())
        } else {
            Ok(last_block.proof_of_work.target)
        }
    }

    fn get_block(&self, index: usize) -> Result<Block, BlockchainError> {
        if index >= self.get_height()? {
            return Err(BlockchainError::BlockNotFound);
        }
        let block_key: StringKey = format!("block_{:010}", index).into();
        Ok(match self.database.get(block_key)? {
            Some(b) => b.try_into()?,
            None => {
                return Err(BlockchainError::Inconsistency);
            }
        })
    }

    fn apply_tx(&mut self, tx: &Transaction, allow_treasury: bool) -> Result<(), BlockchainError> {
        let mut ops = Vec::new();

        let mut acc_src = self.get_account(tx.src.clone())?;

        if tx.src == Address::Treasury && !allow_treasury {
            return Err(BlockchainError::IllegalTreasuryAccess);
        }

        if !tx.verify_signature() {
            return Err(BlockchainError::SignatureError);
        }

        if tx.nonce != acc_src.nonce + 1 {
            return Err(BlockchainError::InvalidTransactionNonce);
        }

        if acc_src.balance < tx.fee {
            return Err(BlockchainError::BalanceInsufficient);
        }

        acc_src.balance -= tx.fee;
        acc_src.nonce += 1;

        match &tx.data {
            TransactionData::RegularSend { dst, amount } => {
                if acc_src.balance < *amount {
                    return Err(BlockchainError::BalanceInsufficient);
                }

                if *dst != tx.src {
                    acc_src.balance -= *amount;
                }

                if *dst != tx.src {
                    let mut acc_dst = self.get_account(dst.clone())?;
                    acc_dst.balance += *amount;

                    ops.push(WriteOp::Put(
                        format!("account_{}", dst).into(),
                        acc_dst.into(),
                    ));
                }
            }
            TransactionData::CreateContract {
                deposit_withdraw_circuit,
                update_circuits,
                state_model,
                initial_state,
            } => {
                ops.push(WriteOp::Put(
                    format!("contract_dw_{}", tx.uid()).into(),
                    deposit_withdraw_circuit.clone().into(),
                ));
                for (i, c) in update_circuits.iter().enumerate() {
                    ops.push(WriteOp::Put(
                        format!("contract_update_{}_{}", tx.uid(), i).into(),
                        c.clone().into(),
                    ));
                }
                ops.push(WriteOp::Put(
                    format!("contract_state_model_{}", tx.uid()).into(),
                    state_model.clone().into(),
                ));
                ops.push(WriteOp::Put(
                    format!("contract_initial_state_{}", tx.uid()).into(),
                    initial_state.clone().into(),
                ));
                let compressed_state =
                    ZkState::new(state_model.clone(), initial_state.clone()).compress();
                ops.push(WriteOp::Put(
                    format!("contract_compressed_state_{}", tx.uid()).into(),
                    compressed_state.into(),
                ));
                unimplemented!();
            }
            TransactionData::DepositWithdraw {
                contract_id: _,
                deposit_withdraws: _,
                next_state: _,
                proof: _,
            } => {
                unimplemented!();
            }
            TransactionData::Update {
                contract_id: _,
                circuit_index: _,
                next_state: _,
                proof: _,
            } => {
                unimplemented!();
            }
        }

        ops.push(WriteOp::Put(
            format!("account_{}", tx.src).into(),
            acc_src.into(),
        ));

        self.database.update(&ops)?;
        Ok(())
    }

    pub fn rollback_block(&mut self) -> Result<(), BlockchainError> {
        let height = self.get_height()?;
        let rollback_key: StringKey = format!("rollback_{:010}", height - 1).into();
        let mut rollback: Vec<WriteOp> = match self.database.get(rollback_key.clone())? {
            Some(b) => b.try_into()?,
            None => {
                return Err(BlockchainError::Inconsistency);
            }
        };
        rollback.push(WriteOp::Remove(format!("block_{:010}", height - 1).into()));
        rollback.push(WriteOp::Remove(format!("merkle_{:010}", height - 1).into()));
        rollback.push(WriteOp::Remove(rollback_key));
        self.database.update(&rollback)?;
        Ok(())
    }

    fn select_transactions(
        &self,
        txs: &[Transaction],
    ) -> Result<Vec<Transaction>, BlockchainError> {
        let mut sorted = txs.to_vec();
        sorted.sort_by(|t1, t2| t1.nonce.cmp(&t2.nonce));
        let mut fork = self.fork_on_ram();
        let mut result = Vec::new();
        for tx in sorted.into_iter() {
            if fork.apply_tx(&tx, false).is_ok() {
                result.push(tx);
            }
        }
        Ok(result)
    }

    fn apply_block(&mut self, block: &Block, draft: bool) -> Result<(), BlockchainError> {
        let curr_height = self.get_height()?;
        let is_genesis = block.header.number == 0;
        let next_reward = self.next_reward()?;

        let pow_key = self.pow_key(block.header.number as usize)?;

        if curr_height > 0 {
            let last_block = self.get_block(curr_height - 1)?;

            if block.header.proof_of_work.timestamp < self.median_timestamp(curr_height - 1)? {
                return Err(BlockchainError::InvalidTimestamp);
            }

            if !draft {
                if !block.header.meets_target(&pow_key) {
                    return Err(BlockchainError::DifficultyTargetUnmet);
                }
            }

            if block.header.number as usize != curr_height {
                return Err(BlockchainError::InvalidBlockNumber);
            }

            if block.header.parent_hash != last_block.header.hash() {
                return Err(BlockchainError::InvalidParentHash);
            }

            if block.header.block_root != block.merkle_tree().root() {
                return Err(BlockchainError::InvalidMerkleRoot);
            }
        }

        let mut fork = self.fork_on_ram();

        // All blocks except genesis block should have a miner reward
        let txs = if !is_genesis {
            let reward_tx = block
                .body
                .first()
                .ok_or(BlockchainError::MinerRewardNotFound)?;

            if reward_tx.src != Address::Treasury
                || reward_tx.fee != 0
                || reward_tx.sig != Signature::Unsigned
            {
                return Err(BlockchainError::InvalidMinerReward);
            }
            match reward_tx.data {
                TransactionData::RegularSend { dst: _, amount } => {
                    if amount != next_reward {
                        return Err(BlockchainError::InvalidMinerReward);
                    }
                }
                _ => {
                    return Err(BlockchainError::InvalidMinerReward);
                }
            }

            // Reward tx allowed to get money from Treasury
            fork.apply_tx(reward_tx, true)?;
            &block.body[1..]
        } else {
            &block.body[..]
        };

        for tx in txs.iter() {
            fork.apply_tx(tx, is_genesis)?; // All genesis block txs are allowed to get from Treasury
        }
        let mut changes = fork.database.to_ops();

        changes.push(WriteOp::Put("height".into(), (curr_height + 1).into()));

        changes.push(WriteOp::Put(
            format!("power_{:010}", block.header.number).into(),
            (block.header.power(&pow_key) + self.get_power()?).into(),
        ));

        changes.push(WriteOp::Put(
            format!("rollback_{:010}", block.header.number).into(),
            self.database.rollback_of(&changes)?.into(),
        ));
        changes.push(WriteOp::Put(
            format!("block_{:010}", block.header.number).into(),
            block.into(),
        ));
        changes.push(WriteOp::Put(
            format!("merkle_{:010}", block.header.number).into(),
            block.merkle_tree().into(),
        ));

        self.database.update(&changes)?;
        Ok(())
    }
}

impl<K: KvStore> Blockchain for KvStoreChain<K> {
    fn get_account(&self, addr: Address) -> Result<Account, BlockchainError> {
        let k = format!("account_{}", addr).into();
        Ok(match self.database.get(k)? {
            Some(b) => b.try_into()?,
            None => Account {
                balance: if addr == Address::Treasury {
                    TOTAL_SUPPLY
                } else {
                    0
                },
                nonce: 0,
            },
        })
    }

    fn will_extend(&self, from: usize, headers: &[Header]) -> Result<bool, BlockchainError> {
        let current_power = self.get_power()?;

        if from == 0 {
            return Err(BlockchainError::ExtendFromGenesis);
        } else if from > self.get_height()? {
            return Err(BlockchainError::ExtendFromFuture);
        }

        let mut new_power: u64 = self
            .database
            .get(format!("power_{:010}", from - 1).into())?
            .ok_or(BlockchainError::Inconsistency)?
            .try_into()?;
        let mut last_header = self.get_block(from - 1)?.header;
        for h in headers.iter() {
            let pow_key = self.pow_key(h.number as usize)?;

            if h.proof_of_work.timestamp < self.median_timestamp(from - 1)? {
                return Err(BlockchainError::InvalidTimestamp);
            }

            if !h.meets_target(&pow_key) {
                return Err(BlockchainError::DifficultyTargetUnmet);
            }

            if h.number != last_header.number + 1 {
                return Err(BlockchainError::InvalidBlockNumber);
            }

            if h.parent_hash != last_header.hash() {
                return Err(BlockchainError::InvalidParentHash);
            }

            last_header = h.clone();
            new_power += h.power(&pow_key);
        }

        Ok(new_power > current_power)
    }
    fn extend(&mut self, from: usize, blocks: &[Block]) -> Result<(), BlockchainError> {
        let curr_height = self.get_height()?;

        if from == 0 {
            return Err(BlockchainError::ExtendFromGenesis);
        } else if from > curr_height {
            return Err(BlockchainError::ExtendFromFuture);
        }

        let mut forked = self.fork_on_ram();

        while forked.get_height()? > from {
            forked.rollback_block()?;
        }

        for block in blocks.iter() {
            forked.apply_block(block, false)?;
        }
        let ops = forked.database.to_ops();

        self.database.update(&ops)?;
        Ok(())
    }
    fn get_height(&self) -> Result<usize, BlockchainError> {
        Ok(match self.database.get("height".into())? {
            Some(b) => b.try_into()?,
            None => 0,
        })
    }
    fn get_headers(
        &self,
        since: usize,
        until: Option<usize>,
    ) -> Result<Vec<Header>, BlockchainError> {
        Ok(self
            .get_blocks(since, until)?
            .into_iter()
            .map(|b| b.header)
            .collect())
    }
    fn get_blocks(
        &self,
        since: usize,
        until: Option<usize>,
    ) -> Result<Vec<Block>, BlockchainError> {
        let mut blks: Vec<Block> = Vec::new();
        let height = self.get_height()?;
        for i in since..until.unwrap_or(height) {
            if i >= height {
                break;
            }
            blks.push(
                self.database
                    .get(format!("block_{:010}", i).into())?
                    .ok_or(BlockchainError::Inconsistency)?
                    .try_into()?,
            );
        }
        Ok(blks)
    }
    fn next_reward(&self) -> Result<Money, BlockchainError> {
        let supply = self.get_account(Address::Treasury)?.balance;
        Ok(supply / config::REWARD_RATIO)
    }
    fn draft_block(
        &self,
        timestamp: u32,
        mempool: &[Transaction],
        wallet: &Wallet,
    ) -> Result<Block, BlockchainError> {
        let height = self.get_height()?;
        let last_block = self.get_block(height - 1)?;
        let treasury_nonce = self.get_account(Address::Treasury)?.nonce;

        let mut txs = vec![Transaction {
            src: Address::Treasury,
            data: TransactionData::RegularSend {
                dst: wallet.get_address(),
                amount: self.next_reward()?,
            },
            nonce: treasury_nonce + 1,
            fee: 0,
            sig: Signature::Unsigned,
        }];
        txs.extend(self.select_transactions(mempool)?);

        let mut blk = Block {
            header: Header {
                parent_hash: last_block.header.hash(),
                number: height as u64,
                block_root: Default::default(),
                proof_of_work: ProofOfWork {
                    timestamp: timestamp,
                    target: self.next_difficulty()?,
                    nonce: 0,
                },
            },
            body: txs,
        };
        blk.header.block_root = blk.merkle_tree().root();
        self.fork_on_ram().apply_block(&blk, true)?; // Check if everything is ok
        Ok(blk)
    }

    fn get_power(&self) -> Result<u64, BlockchainError> {
        let height = self.get_height()?;
        if height == 0 {
            Ok(0)
        } else {
            Ok(self
                .database
                .get(format!("power_{:010}", height - 1).into())?
                .ok_or(BlockchainError::Inconsistency)?
                .try_into()?)
        }
    }

    fn pow_key(&self, index: usize) -> Result<Vec<u8>, BlockchainError> {
        Ok(if index < config::POW_KEY_CHANGE_DELAY {
            config::POW_BASE_KEY.to_vec()
        } else {
            let reference = ((index - config::POW_KEY_CHANGE_DELAY)
                / config::POW_KEY_CHANGE_INTERVAL)
                * config::POW_KEY_CHANGE_INTERVAL;
            self.get_block(reference)?.header.hash().to_vec()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::genesis;
    use crate::core::{Address, TransactionData};
    use crate::db;

    const DEFAULT_DIFFICULTY: u32 = 0x0000ffff;

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
        let pow_key = chain.pow_key(draft.header.number as usize)?;

        if draft.header.meets_target(pow_key.as_slice()) {
            return Ok(());
        }

        draft.header.proof_of_work.nonce = 0;
        while !draft.header.meets_target(pow_key.as_slice()) {
            draft.header.proof_of_work.nonce += 1;
        }

        Ok(())
    }
}
