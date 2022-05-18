use thiserror::Error;

use crate::config;
use crate::config::TOTAL_SUPPLY;
use crate::core::{
    Account, Address, Block, ContractId, Header, Money, ProofOfWork, Signature, Transaction,
    TransactionData,
};
use crate::db::{KvStore, KvStoreError, RamMirrorKvStore, StringKey, WriteOp};
use crate::utils;
use crate::wallet::Wallet;
use crate::zk;
use std::collections::HashMap;

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
    #[error("contract not found")]
    ContractNotFound,
    #[error("update function not found in the given contract")]
    ContractFunctionNotFound,
    #[error("Incorrect zero-knowledge proof")]
    IncorrectZkProof,
    #[error("Full-state not found in the update provided")]
    FullStateNotFound,
    #[error("Invalid full-state in the update provided")]
    FullStateNotValid,
    #[error("cannot draft a new block when full-states are outdated")]
    StatesOutdated,
}

pub trait Blockchain {
    fn get_account(&self, addr: Address) -> Result<Account, BlockchainError>;
    fn next_reward(&self) -> Result<Money, BlockchainError>;
    fn will_extend(&self, from: u64, headers: &[Header]) -> Result<bool, BlockchainError>;
    fn extend(&mut self, from: u64, blocks: &[Block]) -> Result<(), BlockchainError>;
    fn draft_block(
        &self,
        timestamp: u32,
        mempool: &[Transaction],
        wallet: &Wallet,
    ) -> Result<Block, BlockchainError>;
    fn get_height(&self) -> Result<u64, BlockchainError>;
    fn get_headers(&self, since: u64, until: Option<u64>) -> Result<Vec<Header>, BlockchainError>;
    fn get_blocks(&self, since: u64, until: Option<u64>) -> Result<Vec<Block>, BlockchainError>;
    fn get_power(&self) -> Result<u128, BlockchainError>;
    fn pow_key(&self, index: u64) -> Result<Vec<u8>, BlockchainError>;

    fn get_state_height(&self) -> Result<u64, BlockchainError>;
    fn get_contract(&self, contract_id: ContractId) -> Result<zk::ZkContract, BlockchainError>;
    fn get_compressed_state(
        &self,
        contract_id: ContractId,
    ) -> Result<zk::ZkCompressedState, BlockchainError>;
    fn update_states(
        &mut self,
        states: HashMap<ContractId, zk::ZkState>,
    ) -> Result<(), BlockchainError>;
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

    fn median_timestamp(&self, index: u64) -> Result<u32, BlockchainError> {
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

    fn get_block(&self, index: u64) -> Result<Block, BlockchainError> {
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

    fn apply_tx(
        &mut self,
        tx: &Transaction,
        allow_treasury: bool,
    ) -> Result<Option<(ContractId, zk::ZkCompressedState)>, BlockchainError> {
        let mut state_update = None;

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

                    let mut acc_dst = self.get_account(dst.clone())?;
                    acc_dst.balance += *amount;

                    ops.push(WriteOp::Put(
                        format!("account_{}", dst).into(),
                        acc_dst.into(),
                    ));
                }
            }
            TransactionData::CreateContract { contract } => {
                let contract_id = ContractId::new(tx);
                ops.push(WriteOp::Put(
                    format!("contract_{}", contract_id).into(),
                    contract.clone().into(),
                ));
                let compressed_state = contract.initial_state.compress();
                ops.push(WriteOp::Put(
                    format!("contract_compressed_state_{}", contract_id).into(),
                    compressed_state.into(),
                ));
                state_update = Some((contract_id, compressed_state));
            }
            TransactionData::DepositWithdraw {
                contract_id,
                deposit_withdraws: _,
                next_state,
                proof,
            } => {
                let contract = self.get_contract(*contract_id)?;
                let prev_state = self.get_compressed_state(*contract_id)?;
                if !zk::check_proof(&contract.deposit_withdraw, &prev_state, next_state, proof) {
                    return Err(BlockchainError::IncorrectZkProof);
                }
                state_update = Some((*contract_id, *next_state));
                ops.push(WriteOp::Put(
                    format!("contract_compressed_state_{}", contract_id).into(),
                    (*next_state).into(),
                ));
            }
            TransactionData::Update {
                contract_id,
                circuit_index,
                next_state,
                proof,
            } => {
                let contract = self.get_contract(*contract_id)?;
                let prev_state = self.get_compressed_state(*contract_id)?;
                let vk = contract
                    .update
                    .get(*circuit_index as usize)
                    .ok_or(BlockchainError::ContractFunctionNotFound)?;
                if !zk::check_proof(vk, &prev_state, next_state, proof) {
                    return Err(BlockchainError::IncorrectZkProof);
                }
                state_update = Some((*contract_id, *next_state));
                ops.push(WriteOp::Put(
                    format!("contract_compressed_state_{}", contract_id).into(),
                    (*next_state).into(),
                ));
            }
        }

        ops.push(WriteOp::Put(
            format!("account_{}", tx.src).into(),
            acc_src.into(),
        ));

        self.database.update(&ops)?;
        Ok(state_update)
    }

    fn get_changed_states(
        &self,
        index: u64,
    ) -> Result<HashMap<ContractId, zk::ZkCompressedState>, BlockchainError> {
        let k = format!("contract_updates_{:010}", index).into();
        Ok(self
            .database
            .get(k)?
            .map(|b| b.try_into())
            .ok_or(BlockchainError::Inconsistency)??)
    }

    fn get_outdated_states(
        &self,
    ) -> Result<HashMap<ContractId, zk::ZkCompressedState>, BlockchainError> {
        let height = self.get_height()?;
        let state_height = self.get_state_height()?;
        let mut changes = HashMap::new();
        for i in state_height..height {
            let block_changes = self.get_changed_states(i)?;
            changes.extend(block_changes.into_iter());
        }
        Ok(changes)
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
        rollback.push(WriteOp::Remove(
            format!("contract_updates_{:010}", height - 1).into(),
        ));
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

        let pow_key = self.pow_key(block.header.number)?;

        if curr_height > 0 {
            let last_block = self.get_block(curr_height - 1)?;

            if block.header.proof_of_work.timestamp < self.median_timestamp(curr_height - 1)? {
                return Err(BlockchainError::InvalidTimestamp);
            }

            if !draft && !block.header.meets_target(&pow_key) {
                return Err(BlockchainError::DifficultyTargetUnmet);
            }

            if block.header.number != curr_height {
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

        let mut state_updates: HashMap<ContractId, zk::ZkCompressedState> = HashMap::new();
        for tx in txs.iter() {
            // All genesis block txs are allowed to get from Treasury
            if let Some((contract_id, compressed_state)) = fork.apply_tx(tx, is_genesis)? {
                state_updates.insert(contract_id, compressed_state);
            }
        }

        let mut changes = fork.database.to_ops();

        changes.push(WriteOp::Put("height".into(), (curr_height + 1).into()));
        if state_updates.is_empty() && self.get_state_height()? == curr_height {
            changes.push(WriteOp::Put(
                "state_height".into(),
                (curr_height + 1).into(),
            ));
        }

        changes.push(WriteOp::Put(
            format!("power_{:010}", block.header.number).into(),
            (block.header.power() + self.get_power()?).into(),
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
        changes.push(WriteOp::Put(
            format!("contract_updates_{:010}", block.header.number).into(),
            state_updates.into(),
        ));

        self.database.update(&changes)?;
        Ok(())
    }
}

impl<K: KvStore> Blockchain for KvStoreChain<K> {
    fn get_contract(&self, contract_id: ContractId) -> Result<zk::ZkContract, BlockchainError> {
        let k = format!("contract_{}", contract_id).into();
        Ok(self
            .database
            .get(k)?
            .map(|b| b.try_into())
            .ok_or(BlockchainError::ContractNotFound)??)
    }
    fn get_compressed_state(
        &self,
        contract_id: ContractId,
    ) -> Result<zk::ZkCompressedState, BlockchainError> {
        let k = format!("contract_compressed_state_{}", contract_id).into();
        Ok(self
            .database
            .get(k)?
            .map(|b| b.try_into())
            .ok_or(BlockchainError::ContractNotFound)??)
    }

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

    fn will_extend(&self, from: u64, headers: &[Header]) -> Result<bool, BlockchainError> {
        let current_power = self.get_power()?;

        if from == 0 {
            return Err(BlockchainError::ExtendFromGenesis);
        } else if from > self.get_height()? {
            return Err(BlockchainError::ExtendFromFuture);
        }

        let mut new_power: u128 = self
            .database
            .get(format!("power_{:010}", from - 1).into())?
            .ok_or(BlockchainError::Inconsistency)?
            .try_into()?;
        let mut last_header = self.get_block(from - 1)?.header;
        for h in headers.iter() {
            let pow_key = self.pow_key(h.number)?;

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
            new_power += h.power();
        }

        Ok(new_power > current_power)
    }
    fn extend(&mut self, from: u64, blocks: &[Block]) -> Result<(), BlockchainError> {
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
    fn get_height(&self) -> Result<u64, BlockchainError> {
        Ok(match self.database.get("height".into())? {
            Some(b) => b.try_into()?,
            None => 0,
        })
    }
    fn get_state_height(&self) -> Result<u64, BlockchainError> {
        Ok(match self.database.get("state_height".into())? {
            Some(b) => b.try_into()?,
            None => 0,
        })
    }
    fn get_headers(&self, since: u64, until: Option<u64>) -> Result<Vec<Header>, BlockchainError> {
        Ok(self
            .get_blocks(since, until)?
            .into_iter()
            .map(|b| b.header)
            .collect())
    }
    fn get_blocks(&self, since: u64, until: Option<u64>) -> Result<Vec<Block>, BlockchainError> {
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
        let state_height = self.get_state_height()?;

        if height != state_height {
            return Err(BlockchainError::StatesOutdated);
        }

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
                    timestamp,
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

    fn get_power(&self) -> Result<u128, BlockchainError> {
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

    fn pow_key(&self, index: u64) -> Result<Vec<u8>, BlockchainError> {
        Ok(if index < config::POW_KEY_CHANGE_DELAY {
            config::POW_BASE_KEY.to_vec()
        } else {
            let reference = ((index - config::POW_KEY_CHANGE_DELAY)
                / config::POW_KEY_CHANGE_INTERVAL)
                * config::POW_KEY_CHANGE_INTERVAL;
            self.get_block(reference)?.header.hash().to_vec()
        })
    }

    fn update_states(
        &mut self,
        states: HashMap<ContractId, zk::ZkState>,
    ) -> Result<(), BlockchainError> {
        let mut ops = Vec::new();
        for (cid, comp_state) in self.get_outdated_states()? {
            let full_state = states.get(&cid).ok_or(BlockchainError::FullStateNotFound)?;
            if full_state.compress() == comp_state {
                ops.push(WriteOp::Put(
                    format!("contract_state_{}", cid).into(),
                    full_state.into(),
                ));
            } else {
                return Err(BlockchainError::FullStateNotValid);
            }
        }
        ops.push(WriteOp::Put(
            "state_height".into(),
            self.get_height()?.into(),
        ));
        self.database.update(&ops)?;
        Ok(())
    }
}

#[cfg(test)]
mod test;
