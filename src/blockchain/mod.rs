use thiserror::Error;

use crate::core::{
    hash::Hash, Account, Address, Block, ContractAccount, ContractId, ContractUpdate, Hasher,
    Header, Money, PaymentDirection, ProofOfWork, Signature, Transaction, TransactionAndDelta,
    TransactionData, ZkHasher,
};
use crate::db::{KvStore, KvStoreError, RamMirrorKvStore, StringKey, WriteOp};
use crate::utils;
use crate::wallet::Wallet;
use crate::zk;

use rayon::prelude::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone)]
pub struct BlockchainConfig {
    pub genesis: BlockAndPatch,
    pub total_supply: u64,
    pub reward_ratio: u64,
    pub max_delta_size: usize,
    pub block_time: usize,
    pub difficulty_calc_interval: u64,
    pub pow_base_key: &'static [u8],
    pub pow_key_change_delay: u64,
    pub pow_key_change_interval: u64,
    pub median_timestamp_count: u64,
    pub state_manager_config: zk::StateManagerConfig,
}

#[derive(Debug, Clone)]
pub struct TransactionStats {
    pub first_seen: u32,
}

#[derive(Error, Debug)]
pub enum BlockchainError {
    #[error("kvstore error happened: {0}")]
    KvStoreError(#[from] KvStoreError),
    #[error("transaction signature is invalid")]
    SignatureError,
    #[error("balance insufficient")]
    BalanceInsufficient,
    #[error("contract balance insufficient")]
    ContractBalanceInsufficient,
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
    #[error("wrong difficulty target on blocks")]
    DifficultyTargetWrong,
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
    #[error("contract states at requested height are unavailable")]
    StatesUnavailable,
    #[error("block too big")]
    BlockTooBig,
    #[error("compressed-state at specified height not found")]
    CompressedStateNotFound,
    #[error("full-state has invalid deltas")]
    DeltasInvalid,
    #[error("no blocks to roll back")]
    NoBlocksToRollback,
    #[error("zk error happened: {0}")]
    ZkError(#[from] zk::ZkError),
    #[error("state-manager error happened: {0}")]
    StateManagerError(#[from] zk::StateManagerError),
    #[error("invalid deposit/withdraw signature")]
    InvalidDepositWithdrawSignature,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZkBlockchainPatch {
    pub patches: HashMap<ContractId, zk::ZkStatePatch>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZkCompressedStateChange {
    prev_state: zk::ZkCompressedState,
    state: zk::ZkCompressedState,
    prev_height: u64,
}

#[derive(Clone)]
pub struct BlockAndPatch {
    pub block: Block,
    pub patch: ZkBlockchainPatch,
}

pub enum TxSideEffect {
    StateChange {
        contract_id: ContractId,
        state_change: ZkCompressedStateChange,
    },
    Nothing,
}

pub trait Blockchain {
    fn validate_zero_transaction(&self, tx: &zk::ZeroTransaction) -> Result<bool, BlockchainError>;
    fn validate_transaction(&self, tx_delta: &TransactionAndDelta)
        -> Result<bool, BlockchainError>;
    fn get_account(&self, addr: Address) -> Result<Account, BlockchainError>;
    fn get_contract_account(
        &self,
        contract_id: ContractId,
    ) -> Result<ContractAccount, BlockchainError>;
    fn next_reward(&self) -> Result<Money, BlockchainError>;
    fn will_extend(
        &self,
        from: u64,
        headers: &[Header],
        check_pow: bool,
    ) -> Result<bool, BlockchainError>;
    fn extend(&mut self, from: u64, blocks: &[Block]) -> Result<(), BlockchainError>;
    fn rollback(&mut self) -> Result<(), BlockchainError>;
    fn draft_block(
        &self,
        timestamp: u32,
        mempool: &mut HashMap<TransactionAndDelta, TransactionStats>,
        wallet: &Wallet,
        check: bool,
    ) -> Result<BlockAndPatch, BlockchainError>;
    fn get_height(&self) -> Result<u64, BlockchainError>;
    fn get_tip(&self) -> Result<Header, BlockchainError>;
    fn get_headers(&self, since: u64, until: Option<u64>) -> Result<Vec<Header>, BlockchainError>;
    fn get_blocks(&self, since: u64, until: Option<u64>) -> Result<Vec<Block>, BlockchainError>;
    fn get_power(&self) -> Result<u128, BlockchainError>;
    fn pow_key(&self, index: u64) -> Result<Vec<u8>, BlockchainError>;

    fn get_contract(&self, contract_id: ContractId) -> Result<zk::ZkContract, BlockchainError>;

    fn get_outdated_contracts(&self) -> Result<Vec<ContractId>, BlockchainError>;

    fn get_outdated_heights(&self) -> Result<HashMap<ContractId, u64>, BlockchainError>;
    fn generate_state_patch(
        &self,
        heights: HashMap<ContractId, u64>,
        to: <Hasher as Hash>::Output,
    ) -> Result<ZkBlockchainPatch, BlockchainError>;
    fn update_states(&mut self, patch: &ZkBlockchainPatch) -> Result<(), BlockchainError>;
}

pub struct KvStoreChain<K: KvStore> {
    config: BlockchainConfig,
    database: K,
    state_manager: zk::KvStoreStateManager<ZkHasher>,
}

impl<K: KvStore> KvStoreChain<K> {
    pub fn new(database: K, config: BlockchainConfig) -> Result<KvStoreChain<K>, BlockchainError> {
        let mut chain = KvStoreChain::<K> {
            database,
            state_manager: zk::KvStoreStateManager::new(config.state_manager_config.clone()),
            config: config.clone(),
        };
        if chain.get_height()? == 0 {
            chain.apply_block(&config.genesis.block, true)?;
            chain.update_states(&config.genesis.patch)?;
        }
        Ok(chain)
    }

    fn fork_on_ram(&self) -> KvStoreChain<RamMirrorKvStore<'_, K>> {
        KvStoreChain {
            database: self.database.mirror(),
            state_manager: self.state_manager.clone(),
            config: self.config.clone(),
        }
    }

    fn isolated<F, R>(&self, f: F) -> Result<(Vec<WriteOp>, R), BlockchainError>
    where
        F: FnOnce(&mut KvStoreChain<RamMirrorKvStore<'_, K>>) -> Result<R, BlockchainError>,
    {
        let mut mirror = self.fork_on_ram();
        let result = f(&mut mirror)?;
        Ok((mirror.database.to_ops(), result))
    }

    fn median_timestamp(&self, index: u64) -> Result<u32, BlockchainError> {
        Ok(utils::median(
            &(0..std::cmp::min(index + 1, self.config.median_timestamp_count))
                .map(|i| {
                    self.get_header(index - i)
                        .map(|b| b.proof_of_work.timestamp)
                })
                .collect::<Result<Vec<u32>, BlockchainError>>()?,
        ))
    }

    fn next_difficulty(&self) -> Result<u32, BlockchainError> {
        let height = self.get_height()?;
        let last_block = self.get_header(height - 1)?;
        if height % self.config.difficulty_calc_interval == 0 {
            let prev_block = self.get_header(height - self.config.difficulty_calc_interval)?;
            Ok(utils::calc_pow_difficulty(
                self.config.difficulty_calc_interval,
                self.config.block_time,
                &last_block.proof_of_work,
                &prev_block.proof_of_work,
            ))
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

    fn get_compressed_state_at(
        &self,
        contract_id: ContractId,
        index: u64,
    ) -> Result<zk::ZkCompressedState, BlockchainError> {
        let state_model = self.get_contract(contract_id)?.state_model;
        if index >= self.get_contract_account(contract_id)?.height {
            return Err(BlockchainError::CompressedStateNotFound);
        }
        if index == 0 {
            return Ok(zk::ZkCompressedState::empty::<ZkHasher>(state_model));
        }
        let header_key: StringKey =
            format!("contract_compressed_state_{}_{}", contract_id, index).into();
        Ok(match self.database.get(header_key)? {
            Some(b) => b.try_into()?,
            None => {
                return Err(BlockchainError::Inconsistency);
            }
        })
    }

    fn get_header(&self, index: u64) -> Result<Header, BlockchainError> {
        if index >= self.get_height()? {
            return Err(BlockchainError::BlockNotFound);
        }
        let header_key: StringKey = format!("header_{:010}", index).into();
        Ok(match self.database.get(header_key)? {
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
    ) -> Result<TxSideEffect, BlockchainError> {
        let (ops, side_effect) = self.isolated(|chain| {
            let mut side_effect = TxSideEffect::Nothing;

            let mut acc_src = chain.get_account(tx.src.clone())?;

            if tx.src == Address::Treasury && !allow_treasury {
                return Err(BlockchainError::IllegalTreasuryAccess);
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

                        let mut acc_dst = chain.get_account(dst.clone())?;
                        acc_dst.balance += *amount;

                        chain.database.update(&[WriteOp::Put(
                            format!("account_{}", dst).into(),
                            acc_dst.into(),
                        )])?;
                    }
                }
                TransactionData::CreateContract { contract } => {
                    let contract_id = ContractId::new(tx);
                    chain.database.update(&[WriteOp::Put(
                        format!("contract_{}", contract_id).into(),
                        contract.clone().into(),
                    )])?;
                    let compressed_empty =
                        zk::ZkCompressedState::empty::<ZkHasher>(contract.state_model.clone());
                    chain.database.update(&[WriteOp::Put(
                        format!("contract_account_{}", contract_id).into(),
                        ContractAccount {
                            compressed_state: contract.initial_state,
                            balance: 0,
                            height: 1,
                            nonce: 0,
                        }
                        .into(),
                    )])?;
                    chain.database.update(&[WriteOp::Put(
                        format!("contract_compressed_state_{}_{}", contract_id, 1).into(),
                        contract.initial_state.into(),
                    )])?;
                    side_effect = TxSideEffect::StateChange {
                        contract_id,
                        state_change: ZkCompressedStateChange {
                            prev_height: 0,
                            prev_state: compressed_empty,
                            state: contract.initial_state,
                        },
                    };
                }
                TransactionData::UpdateContract {
                    contract_id,
                    updates,
                } => {
                    let contract = chain.get_contract(*contract_id)?;

                    for update in updates {
                        let prev_account = chain.get_contract_account(*contract_id)?;
                        let mut new_account = prev_account.clone();
                        new_account.height += 1;

                        let (circuit, aux_data, next_state, proof) = match update {
                            ContractUpdate::DepositWithdraw {
                                deposit_withdraws,
                                next_state,
                                proof,
                            } => {
                                let circuit = &contract.deposit_withdraw_function;
                                let state_model = zk::ZkStateModel::List {
                                    item_type: Box::new(zk::ZkStateModel::Struct {
                                        field_types: vec![
                                            zk::ZkStateModel::Scalar, // Pub-key
                                            zk::ZkStateModel::Scalar, // Amount
                                        ],
                                    }),
                                    log4_size: contract.log4_deposit_withdraw_capacity,
                                };
                                let mut state_builder =
                                    zk::ZkStateBuilder::<ZkHasher>::new(state_model);
                                for (i, dw) in deposit_withdraws.iter().enumerate() {
                                    // Set amount
                                    state_builder.set(
                                        zk::ZkDataLocator(vec![i as u32, 1]),
                                        zk::ZkScalar::from(dw.amount),
                                    )?;

                                    let mut addr_account = chain
                                        .get_account(Address::PublicKey(dw.address.clone()))?;
                                    match &dw.direction {
                                        PaymentDirection::Deposit(_) => {
                                            if addr_account.nonce != dw.nonce {
                                                return Err(
                                                    BlockchainError::InvalidTransactionNonce,
                                                );
                                            }
                                            if addr_account.balance < dw.amount {
                                                return Err(BlockchainError::BalanceInsufficient);
                                            }
                                            addr_account.balance -= dw.amount;
                                            addr_account.nonce += 1;

                                            new_account.balance += dw.amount;
                                        }
                                        PaymentDirection::Withdraw(_) => {
                                            if new_account.nonce != dw.nonce {
                                                return Err(
                                                    BlockchainError::InvalidTransactionNonce,
                                                );
                                            }
                                            if new_account.balance < dw.amount {
                                                return Err(
                                                    BlockchainError::ContractBalanceInsufficient,
                                                );
                                            }
                                            new_account.balance -= dw.amount;
                                            new_account.nonce += 1;

                                            addr_account.balance += dw.amount;
                                        }
                                    }

                                    chain.database.update(&[WriteOp::Put(
                                        format!(
                                            "account_{}",
                                            Address::PublicKey(dw.address.clone())
                                        )
                                        .into(),
                                        addr_account.into(),
                                    )])?;

                                    if !dw.verify_signature() {
                                        return Err(
                                            BlockchainError::InvalidDepositWithdrawSignature,
                                        );
                                    }
                                }
                                let aux_data = state_builder.compress()?;
                                (circuit, aux_data, next_state, proof)
                            }
                            ContractUpdate::FunctionCall {
                                function_id,
                                next_state,
                                proof,
                            } => {
                                let circuit = contract
                                    .functions
                                    .get(*function_id as usize)
                                    .ok_or(BlockchainError::ContractFunctionNotFound)?;
                                let aux_data = zk::ZkCompressedState::default();
                                (circuit, aux_data, next_state, proof)
                            }
                        };

                        if !zk::check_proof(
                            circuit,
                            &prev_account.compressed_state,
                            &aux_data,
                            next_state,
                            proof,
                        ) {
                            return Err(BlockchainError::IncorrectZkProof);
                        }

                        new_account.compressed_state = *next_state;

                        chain.database.update(&[WriteOp::Put(
                            format!("contract_account_{}", contract_id).into(),
                            new_account.clone().into(),
                        )])?;
                        chain.database.update(&[WriteOp::Put(
                            format!(
                                "contract_compressed_state_{}_{}",
                                contract_id, new_account.height
                            )
                            .into(),
                            (*next_state).into(),
                        )])?;
                        side_effect = TxSideEffect::StateChange {
                            contract_id: *contract_id,
                            state_change: ZkCompressedStateChange {
                                prev_height: prev_account.height,
                                prev_state: prev_account.compressed_state,
                                state: *next_state,
                            },
                        };
                    }
                }
            }

            chain.database.update(&[WriteOp::Put(
                format!("account_{}", tx.src).into(),
                acc_src.into(),
            )])?;

            Ok(side_effect)
        })?;

        self.database.update(&ops)?;
        Ok(side_effect)
    }

    fn get_changed_states(
        &self,
        index: u64,
    ) -> Result<HashMap<ContractId, ZkCompressedStateChange>, BlockchainError> {
        let k = format!("contract_updates_{:010}", index).into();
        Ok(self
            .database
            .get(k)?
            .map(|b| b.try_into())
            .ok_or(BlockchainError::Inconsistency)??)
    }

    fn select_transactions(
        &self,
        txs: &mut HashMap<TransactionAndDelta, TransactionStats>,
        check: bool,
    ) -> Result<Vec<TransactionAndDelta>, BlockchainError> {
        let mut sorted = txs.keys().cloned().collect::<Vec<_>>();
        sorted.sort_by(|t1, t2| t1.tx.nonce.cmp(&t2.tx.nonce));
        let (_, result) = self.isolated(|chain| {
            let mut result = Vec::new();
            let mut sz = 0isize;
            for tx in sorted.into_iter() {
                let delta =
                    tx.tx.size() as isize + tx.state_delta.clone().unwrap_or_default().size();
                if !check
                    || (sz + delta <= chain.config.max_delta_size as isize
                        && tx.tx.verify_signature()
                        && chain.apply_tx(&tx.tx, false).is_ok())
                {
                    txs.remove(&tx);
                    sz += delta;
                    result.push(tx);
                }
            }
            Ok(result)
        })?;
        Ok(result)
    }

    fn apply_block(&mut self, block: &Block, check_pow: bool) -> Result<(), BlockchainError> {
        let (ops, _) = self.isolated(|chain| {
            let curr_height = chain.get_height()?;
            let is_genesis = block.header.number == 0;
            let next_reward = chain.next_reward()?;

            if curr_height > 0 {
                if block.merkle_tree().root() != block.header.block_root {
                    return Err(BlockchainError::InvalidMerkleRoot);
                }

                chain.will_extend(curr_height, &[block.header.clone()], check_pow)?;
            }

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
                chain.apply_tx(reward_tx, true)?;
                &block.body[1..]
            } else {
                &block.body[..]
            };

            let mut body_size = 0usize;
            let mut state_size_delta = 0isize;
            let mut state_updates: HashMap<ContractId, ZkCompressedStateChange> = HashMap::new();
            let mut outdated_contracts = self.get_outdated_contracts()?;

            if !txs.par_iter().all(|tx| tx.verify_signature()) {
                return Err(BlockchainError::SignatureError);
            }

            for tx in txs.iter() {
                body_size += tx.size();
                // All genesis block txs are allowed to get from Treasury
                if let TxSideEffect::StateChange {
                    contract_id,
                    state_change,
                } = chain.apply_tx(tx, is_genesis)?
                {
                    state_size_delta += state_change.state.size() as isize
                        - state_change.prev_state.size() as isize;
                    state_updates.insert(contract_id, state_change.clone());
                    outdated_contracts.push(contract_id);
                }
            }

            if (body_size as isize + state_size_delta) as usize > self.config.max_delta_size {
                return Err(BlockchainError::BlockTooBig);
            }

            chain.database.update(&[
                WriteOp::Put("height".into(), (curr_height + 1).into()),
                WriteOp::Put(
                    format!("power_{:010}", block.header.number).into(),
                    (block.header.power() + self.get_power()?).into(),
                ),
            ])?;

            let rollback = chain.database.rollback()?;

            chain.database.update(&[
                WriteOp::Put(
                    format!("rollback_{:010}", block.header.number).into(),
                    rollback.into(),
                ),
                WriteOp::Put(
                    format!("header_{:010}", block.header.number).into(),
                    block.header.clone().into(),
                ),
                WriteOp::Put(
                    format!("block_{:010}", block.header.number).into(),
                    block.into(),
                ),
                WriteOp::Put(
                    format!("merkle_{:010}", block.header.number).into(),
                    block.merkle_tree().into(),
                ),
                WriteOp::Put(
                    format!("contract_updates_{:010}", block.header.number).into(),
                    state_updates.into(),
                ),
                if outdated_contracts.is_empty() {
                    WriteOp::Remove("outdated".into())
                } else {
                    WriteOp::Put("outdated".into(), outdated_contracts.clone().into())
                },
            ])?;

            Ok(())
        })?;

        self.database.update(&ops)?;
        Ok(())
    }
}

impl<K: KvStore> Blockchain for KvStoreChain<K> {
    fn rollback(&mut self) -> Result<(), BlockchainError> {
        let (ops, _) = self.isolated(|chain| {
            let height = chain.get_height()?;

            if height == 0 {
                return Err(BlockchainError::NoBlocksToRollback);
            }

            let rollback_key: StringKey = format!("rollback_{:010}", height - 1).into();
            let rollback: Vec<WriteOp> = match chain.database.get(rollback_key.clone())? {
                Some(b) => b.try_into()?,
                None => {
                    return Err(BlockchainError::Inconsistency);
                }
            };

            let mut outdated = chain.get_outdated_contracts()?;
            let changed_states = chain.get_changed_states(height - 1)?;

            for (cid, comp) in changed_states {
                if comp.prev_height == 0 {
                    chain
                        .state_manager
                        .delete_contract(&mut chain.database, cid)?;
                    outdated.retain(|&x| x != cid);
                    continue;
                }

                if !outdated.contains(&cid) {
                    let (ops, result) = chain.isolated(|fork| {
                        Ok(fork
                            .state_manager
                            .rollback_contract(&mut fork.database, cid)?)
                    })?;

                    if result != Some(comp.prev_state) && comp.prev_height > 0 {
                        outdated.push(cid);
                    } else {
                        chain.database.update(&ops)?;
                    }
                } else {
                    let local_compressed_state = chain.state_manager.root(&chain.database, cid)?;
                    if local_compressed_state == comp.prev_state {
                        outdated.retain(|&x| x != cid);
                    }
                }
            }

            chain.database.update(&rollback)?;
            chain.database.update(&[
                if outdated.is_empty() {
                    WriteOp::Remove("outdated".into())
                } else {
                    WriteOp::Put("outdated".into(), outdated.clone().into())
                },
                WriteOp::Remove(format!("header_{:010}", height - 1).into()),
                WriteOp::Remove(format!("block_{:010}", height - 1).into()),
                WriteOp::Remove(format!("merkle_{:010}", height - 1).into()),
                WriteOp::Remove(format!("contract_updates_{:010}", height - 1).into()),
                WriteOp::Remove(rollback_key),
            ])?;

            Ok(())
        })?;
        self.database.update(&ops)?;
        Ok(())
    }

    fn get_outdated_heights(&self) -> Result<HashMap<ContractId, u64>, BlockchainError> {
        let outdated = self.get_outdated_contracts()?;
        let mut ret = HashMap::new();
        for cid in outdated {
            let state_height = self.state_manager.height_of(&self.database, cid)?;
            ret.insert(cid, state_height);
        }
        Ok(ret)
    }
    fn get_outdated_contracts(&self) -> Result<Vec<ContractId>, BlockchainError> {
        Ok(match self.database.get("outdated".into())? {
            Some(b) => {
                let val: Vec<ContractId> = b.try_into()?;
                if val.is_empty() {
                    return Err(BlockchainError::Inconsistency);
                }
                val
            }
            None => Vec::new(),
        })
    }
    fn get_tip(&self) -> Result<Header, BlockchainError> {
        self.get_header(self.get_height()? - 1)
    }
    fn get_contract(&self, contract_id: ContractId) -> Result<zk::ZkContract, BlockchainError> {
        let k = format!("contract_{}", contract_id).into();
        Ok(self
            .database
            .get(k)?
            .map(|b| b.try_into())
            .ok_or(BlockchainError::ContractNotFound)??)
    }
    fn get_contract_account(
        &self,
        contract_id: ContractId,
    ) -> Result<ContractAccount, BlockchainError> {
        let k = format!("contract_account_{}", contract_id).into();
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
                    self.config.total_supply
                } else {
                    0
                },
                nonce: 0,
            },
        })
    }

    fn will_extend(
        &self,
        from: u64,
        headers: &[Header],
        check_pow: bool,
    ) -> Result<bool, BlockchainError> {
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

        let mut last_header = self.get_header(from - 1)?;
        let mut last_pow = self
            .get_header(
                last_header.number - (last_header.number % self.config.difficulty_calc_interval),
            )?
            .proof_of_work;

        for h in headers.iter() {
            if h.number % self.config.difficulty_calc_interval == 0 {
                if h.proof_of_work.target
                    != utils::calc_pow_difficulty(
                        self.config.difficulty_calc_interval,
                        self.config.block_time,
                        &last_header.proof_of_work,
                        &last_pow,
                    )
                {
                    return Err(BlockchainError::DifficultyTargetWrong);
                }
                last_pow = h.proof_of_work;
            }

            let pow_key = self.pow_key(h.number)?;

            if h.proof_of_work.timestamp < self.median_timestamp(from - 1)? {
                return Err(BlockchainError::InvalidTimestamp);
            }

            if last_pow.target != h.proof_of_work.target {
                return Err(BlockchainError::DifficultyTargetWrong);
            }

            if check_pow && !h.meets_target(&pow_key) {
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
        let (ops, _) = self.isolated(|chain| {
            let curr_height = chain.get_height()?;

            if from == 0 {
                return Err(BlockchainError::ExtendFromGenesis);
            } else if from > curr_height {
                return Err(BlockchainError::ExtendFromFuture);
            }

            while chain.get_height()? > from {
                chain.rollback()?;
            }

            for block in blocks.iter() {
                chain.apply_block(block, true)?;
            }

            Ok(())
        })?;

        self.database.update(&ops)?;
        Ok(())
    }
    fn get_height(&self) -> Result<u64, BlockchainError> {
        Ok(match self.database.get("height".into())? {
            Some(b) => b.try_into()?,
            None => 0,
        })
    }
    fn get_headers(&self, since: u64, until: Option<u64>) -> Result<Vec<Header>, BlockchainError> {
        let mut blks: Vec<Header> = Vec::new();
        let height = self.get_height()?;
        for i in since..until.unwrap_or(height) {
            if i >= height {
                break;
            }
            blks.push(self.get_header(i)?);
        }
        Ok(blks)
    }
    fn get_blocks(&self, since: u64, until: Option<u64>) -> Result<Vec<Block>, BlockchainError> {
        let mut blks: Vec<Block> = Vec::new();
        let height = self.get_height()?;
        for i in since..until.unwrap_or(height) {
            if i >= height {
                break;
            }
            blks.push(self.get_block(i)?);
        }
        Ok(blks)
    }
    fn next_reward(&self) -> Result<Money, BlockchainError> {
        let supply = self.get_account(Address::Treasury)?.balance;
        Ok(supply / self.config.reward_ratio)
    }
    fn draft_block(
        &self,
        timestamp: u32,
        mempool: &mut HashMap<TransactionAndDelta, TransactionStats>,
        wallet: &Wallet,
        check: bool,
    ) -> Result<BlockAndPatch, BlockchainError> {
        let height = self.get_height()?;
        let outdated_contracts = self.get_outdated_contracts()?;

        if !outdated_contracts.is_empty() {
            return Err(BlockchainError::StatesOutdated);
        }

        let last_header = self.get_header(height - 1)?;
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

        let tx_and_deltas = self.select_transactions(mempool, check)?;
        let mut block_delta: HashMap<ContractId, zk::ZkStatePatch> = HashMap::new();
        for tx_delta in tx_and_deltas.iter() {
            if let Some(contract_id) = match &tx_delta.tx.data {
                TransactionData::CreateContract { .. } => Some(ContractId::new(&tx_delta.tx)),
                TransactionData::UpdateContract { contract_id, .. } => Some(*contract_id),
                _ => None,
            } {
                block_delta.insert(
                    contract_id,
                    zk::ZkStatePatch::Delta(
                        tx_delta
                            .state_delta
                            .clone()
                            .ok_or(BlockchainError::FullStateNotFound)?,
                    ),
                );
            }
        }
        let block_delta = ZkBlockchainPatch {
            patches: block_delta,
        };

        txs.extend(tx_and_deltas.iter().map(|tp| tp.tx.clone()));

        let mut blk = Block {
            header: Header {
                parent_hash: last_header.hash(),
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

        self.isolated(|chain| {
            chain.apply_block(&blk, false)?; // Check if everything is ok
            chain.update_states(&block_delta)?;

            Ok(())
        })?;

        Ok(BlockAndPatch {
            block: blk,
            patch: block_delta,
        })
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
        Ok(if index < self.config.pow_key_change_delay {
            self.config.pow_base_key.to_vec()
        } else {
            let reference = ((index - self.config.pow_key_change_delay)
                / self.config.pow_key_change_interval)
                * self.config.pow_key_change_interval;
            self.get_header(reference)?.hash().to_vec()
        })
    }

    fn update_states(&mut self, patch: &ZkBlockchainPatch) -> Result<(), BlockchainError> {
        let (ops, _) = self.isolated(|chain| {
            let mut outdated_contracts = chain.get_outdated_contracts()?;

            for cid in outdated_contracts.clone() {
                let contract_account = chain.get_contract_account(cid)?;
                let patch = patch
                    .patches
                    .get(&cid)
                    .ok_or(BlockchainError::FullStateNotFound)?;
                match &patch {
                    zk::ZkStatePatch::Full(full) => {
                        let (_, rollback_results) = chain.state_manager.reset_contract(
                            &mut chain.database,
                            cid,
                            contract_account.height,
                            full,
                        )?;
                        for (i, rollback_result) in rollback_results.into_iter().enumerate() {
                            if rollback_result
                                != self.get_compressed_state_at(
                                    cid,
                                    contract_account.height - 1 - i as u64,
                                )?
                            {
                                return Err(BlockchainError::DeltasInvalid);
                            }
                        }
                    }
                    zk::ZkStatePatch::Delta(delta) => {
                        chain
                            .state_manager
                            .update_contract(&mut chain.database, cid, delta)?;
                    }
                };

                if chain.state_manager.root(&chain.database, cid)?
                    != contract_account.compressed_state
                {
                    return Err(BlockchainError::FullStateNotValid);
                }
                outdated_contracts.retain(|&x| x != cid);
            }

            chain.database.update(&[if outdated_contracts.is_empty() {
                WriteOp::Remove("outdated".into())
            } else {
                WriteOp::Put("outdated".into(), outdated_contracts.clone().into())
            }])?;

            Ok(())
        })?;
        self.database.update(&ops)?;
        Ok(())
    }

    fn validate_zero_transaction(
        &self,
        _tx: &zk::ZeroTransaction,
    ) -> Result<bool, BlockchainError> {
        Ok(true)
    }

    fn validate_transaction(
        &self,
        tx_delta: &TransactionAndDelta,
    ) -> Result<bool, BlockchainError> {
        Ok(
            self.get_account(tx_delta.tx.src.clone())?.balance > 0
                && tx_delta.tx.verify_signature(),
        )
    }
    fn generate_state_patch(
        &self,
        heights: HashMap<ContractId, u64>,
        to: <Hasher as Hash>::Output,
    ) -> Result<ZkBlockchainPatch, BlockchainError> {
        let height = self.get_height()?;
        let last_header = self.get_header(height - 1)?;

        if last_header.hash() != to {
            return Err(BlockchainError::StatesUnavailable);
        }

        let outdated_contracts = self.get_outdated_contracts()?;

        let mut blockchain_patch = ZkBlockchainPatch {
            patches: HashMap::new(),
        };
        for (cid, height) in heights {
            if !outdated_contracts.contains(&cid) {
                let away = self.state_manager.height_of(&self.database, cid)? - height;
                blockchain_patch.patches.insert(
                    cid,
                    if let Some(delta) = self.state_manager.delta_of(&self.database, cid, away)? {
                        zk::ZkStatePatch::Delta(delta)
                    } else {
                        zk::ZkStatePatch::Full(
                            self.state_manager.get_full_state(&self.database, cid)?,
                        )
                    },
                );
            }
        }

        Ok(blockchain_patch)
    }
}

#[cfg(test)]
mod test;
