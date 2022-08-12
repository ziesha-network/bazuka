mod error;
pub use error::*;

use crate::config::blockchain::MPN_CONTRACT_ID;
use crate::core::{
    hash::Hash, Account, Address, Block, ContractAccount, ContractId, ContractPayment,
    ContractUpdate, Hasher, Header, Money, PaymentDirection, ProofOfWork, Signature, Transaction,
    TransactionAndDelta, TransactionData, ZkHasher,
};
use crate::db::{keys, KvStore, RamMirrorKvStore, WriteOp};
use crate::utils;
use crate::wallet::Wallet;
use crate::zk;

use rayon::prelude::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::consensus::pow::Difficulty;

#[derive(Clone)]
pub struct BlockchainConfig {
    pub genesis: BlockAndPatch,
    pub total_supply: u64,
    pub reward_ratio: u64,
    pub max_block_size: usize,
    pub max_delta_size: usize,
    pub block_time: usize,
    pub difficulty_calc_interval: u64,
    pub pow_base_key: &'static [u8],
    pub pow_key_change_delay: u64,
    pub pow_key_change_interval: u64,
    pub median_timestamp_count: u64,
    pub mpn_num_function_calls: usize,
    pub mpn_num_contract_payments: usize,
    pub minimum_pow_difficulty: Difficulty,
}

#[derive(Debug, Clone)]
pub struct TransactionStats {
    pub first_seen: u32,
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
    fn config(&self) -> &BlockchainConfig;

    fn cleanup_mempool(
        &self,
        mempool: &mut HashMap<TransactionAndDelta, TransactionStats>,
    ) -> Result<(), BlockchainError>;
    fn cleanup_contract_payment_mempool(
        &self,
        mempool: &mut HashMap<ContractPayment, TransactionStats>,
    ) -> Result<(), BlockchainError>;
    fn validate_zero_transaction(&self, tx: &zk::ZeroTransaction) -> Result<bool, BlockchainError>;
    fn validate_contract_payment(&self, tx: &ContractPayment) -> Result<bool, BlockchainError>;
    fn validate_transaction(&self, tx_delta: &TransactionAndDelta)
        -> Result<bool, BlockchainError>;
    fn get_account(&self, addr: Address) -> Result<Account, BlockchainError>;
    fn get_contract_account(
        &self,
        contract_id: ContractId,
    ) -> Result<ContractAccount, BlockchainError>;
    fn read_state(
        &self,
        contract_id: ContractId,
        locator: zk::ZkDataLocator,
    ) -> Result<zk::ZkScalar, BlockchainError>;
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
        mempool: &HashMap<TransactionAndDelta, TransactionStats>,
        wallet: &Wallet,
        check: bool,
    ) -> Result<Option<BlockAndPatch>, BlockchainError>;
    fn get_height(&self) -> Result<u64, BlockchainError>;
    fn get_tip(&self) -> Result<Header, BlockchainError>;
    fn get_headers(&self, since: u64, count: u64) -> Result<Vec<Header>, BlockchainError>;
    fn get_blocks(&self, since: u64, count: u64) -> Result<Vec<Block>, BlockchainError>;
    fn get_header(&self, index: u64) -> Result<Header, BlockchainError>;
    fn get_block(&self, index: u64) -> Result<Block, BlockchainError>;
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
}

impl<K: KvStore> KvStoreChain<K> {
    pub fn new(database: K, config: BlockchainConfig) -> Result<KvStoreChain<K>, BlockchainError> {
        let mut chain = KvStoreChain::<K> {
            database,
            config: config.clone(),
        };
        if chain.get_height()? == 0 {
            chain.apply_block(&config.genesis.block, true)?;
            chain.update_states(&config.genesis.patch)?;
        } else {
            if config.genesis.block != chain.get_block(0)? {
                return Err(BlockchainError::DifferentGenesis);
            }
        }
        Ok(chain)
    }

    fn fork_on_ram(&self) -> KvStoreChain<RamMirrorKvStore<'_, K>> {
        KvStoreChain {
            database: self.database.mirror(),
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

    fn next_difficulty(&self) -> Result<Difficulty, BlockchainError> {
        let height = self.get_height()?;
        let last_block = self.get_header(height - 1)?;
        if height % self.config.difficulty_calc_interval == 0 {
            let prev_block = self.get_header(height - self.config.difficulty_calc_interval)?;
            Ok(utils::calc_pow_difficulty(
                self.config.difficulty_calc_interval,
                self.config.block_time,
                self.config.minimum_pow_difficulty,
                &last_block.proof_of_work,
                &prev_block.proof_of_work,
            ))
        } else {
            Ok(last_block.proof_of_work.target)
        }
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
        Ok(
            match self
                .database
                .get(keys::compressed_state_at(&contract_id, index))?
            {
                Some(b) => b.try_into()?,
                None => {
                    return Err(BlockchainError::Inconsistency);
                }
            },
        )
    }

    fn apply_contract_payment(
        &mut self,
        contract_payment: &ContractPayment,
        in_memory_account: Option<&mut ContractAccount>,
    ) -> Result<(), BlockchainError> {
        let (ops, _) = self.isolated(|chain| {
            if !contract_payment.verify_signature() {
                return Err(BlockchainError::InvalidContractPaymentSignature);
            }

            // Actual contract-account that exists on DB
            let mut db_account = chain.get_contract_account(contract_payment.contract_id)?;

            // If a mutable contract-account is supplied already, we will reflect our changes on
            // that instead of the db
            let (contract_account, should_update_contract) =
                if let Some(in_memory_account) = in_memory_account {
                    (in_memory_account, false)
                } else {
                    (&mut db_account, true)
                };

            let mut addr_account =
                chain.get_account(Address::PublicKey(contract_payment.address.clone()))?;
            if contract_payment.nonce != addr_account.nonce + 1 {
                return Err(BlockchainError::InvalidTransactionNonce);
            }
            addr_account.nonce += 1;
            match &contract_payment.direction {
                PaymentDirection::Deposit(_) => {
                    if addr_account.balance < contract_payment.amount + contract_payment.fee {
                        return Err(BlockchainError::BalanceInsufficient);
                    }
                    addr_account.balance -= contract_payment.amount + contract_payment.fee;
                    contract_account.balance += contract_payment.amount;
                }
                PaymentDirection::Withdraw(_) => {
                    if contract_account.balance < contract_payment.amount + contract_payment.fee {
                        return Err(BlockchainError::ContractBalanceInsufficient);
                    }
                    contract_account.balance -= contract_payment.amount + contract_payment.fee;
                    addr_account.balance += contract_payment.amount;
                }
            }
            if should_update_contract {
                chain.database.update(&[WriteOp::Put(
                    keys::contract_account(&contract_payment.contract_id),
                    contract_account.clone().into(),
                )])?;
            }
            chain.database.update(&[WriteOp::Put(
                keys::account(&Address::PublicKey(contract_payment.address.clone())),
                addr_account.into(),
            )])?;
            Ok(())
        })?;
        self.database.update(&ops)?;
        Ok(())
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

                        chain
                            .database
                            .update(&[WriteOp::Put(keys::account(dst), acc_dst.into())])?;
                    }
                }
                TransactionData::CreateContract { contract } => {
                    let contract_id = ContractId::new(tx);
                    chain.database.update(&[WriteOp::Put(
                        keys::contract(&contract_id),
                        contract.clone().into(),
                    )])?;
                    let compressed_empty =
                        zk::ZkCompressedState::empty::<ZkHasher>(contract.state_model.clone());
                    chain.database.update(&[WriteOp::Put(
                        keys::contract_account(&contract_id),
                        ContractAccount {
                            compressed_state: contract.initial_state,
                            balance: 0,
                            height: 1,
                        }
                        .into(),
                    )])?;
                    chain.database.update(&[WriteOp::Put(
                        keys::compressed_state_at(&contract_id, 1),
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
                    let mut executor_fee = 0;

                    for update in updates {
                        let prev_account = chain.get_contract_account(*contract_id)?;
                        let mut new_account = prev_account.clone();
                        new_account.height += 1;

                        let (circuit, aux_data, next_state, proof) = match update {
                            ContractUpdate::Payment {
                                payments,
                                next_state,
                                proof,
                            } => {
                                let circuit = &contract.payment_function;
                                let state_model = zk::ZkStateModel::List {
                                    item_type: Box::new(zk::CONTRACT_PAYMENT_STATE_MODEL.clone()),
                                    log4_size: contract.log4_payment_capacity,
                                };
                                let mut state_builder =
                                    zk::ZkStateBuilder::<ZkHasher>::new(state_model);
                                for (i, contract_payment) in payments.iter().enumerate() {
                                    executor_fee += contract_payment.fee;
                                    let pk = contract_payment.zk_address.0.decompress();
                                    state_builder.batch_set(&zk::ZkDeltaPairs(
                                        [
                                            (
                                                zk::ZkDataLocator(vec![i as u32, 0]),
                                                Some(zk::ZkScalar::from(
                                                    contract_payment.zk_address_index as u64,
                                                )),
                                            ),
                                            (
                                                zk::ZkDataLocator(vec![i as u32, 1]),
                                                Some(zk::ZkScalar::from(
                                                    match contract_payment.direction {
                                                        PaymentDirection::Deposit(_) => {
                                                            contract_payment.amount as u64
                                                        }
                                                        PaymentDirection::Withdraw(_) => {
                                                            (-(contract_payment.amount as i64))
                                                                as u64
                                                        }
                                                    },
                                                )),
                                            ),
                                            (
                                                zk::ZkDataLocator(vec![i as u32, 2]),
                                                Some(zk::ZkScalar::from(pk.0)),
                                            ),
                                            (
                                                zk::ZkDataLocator(vec![i as u32, 3]),
                                                Some(zk::ZkScalar::from(pk.1)),
                                            ),
                                        ]
                                        .into(),
                                    ))?;

                                    chain.apply_contract_payment(
                                        contract_payment,
                                        Some(&mut new_account),
                                    )?;
                                }
                                let aux_data = state_builder.compress()?;
                                (circuit, aux_data, next_state, proof)
                            }
                            ContractUpdate::FunctionCall {
                                function_id,
                                next_state,
                                proof,
                                fee,
                            } => {
                                executor_fee += fee;
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
                        acc_src.balance += executor_fee; // Pay executor fee

                        chain.database.update(&[WriteOp::Put(
                            keys::contract_account(contract_id),
                            new_account.clone().into(),
                        )])?;
                        chain.database.update(&[WriteOp::Put(
                            keys::compressed_state_at(contract_id, new_account.height),
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

            chain
                .database
                .update(&[WriteOp::Put(keys::account(&tx.src), acc_src.into())])?;

            // Fees go to the Treasury account first
            if tx.src != Address::Treasury {
                let mut acc_treasury = chain.get_account(Address::Treasury)?;
                acc_treasury.balance += tx.fee;
                chain.database.update(&[WriteOp::Put(
                    keys::account(&Address::Treasury),
                    acc_treasury.into(),
                )])?;
            }

            Ok(side_effect)
        })?;

        self.database.update(&ops)?;
        Ok(side_effect)
    }

    fn get_changed_states(
        &self,
        index: u64,
    ) -> Result<HashMap<ContractId, ZkCompressedStateChange>, BlockchainError> {
        Ok(self
            .database
            .get(keys::contract_updates(index))?
            .map(|b| b.try_into())
            .ok_or(BlockchainError::Inconsistency)??)
    }

    fn select_transactions(
        &self,
        txs: &HashMap<TransactionAndDelta, TransactionStats>,
        check: bool,
    ) -> Result<Vec<TransactionAndDelta>, BlockchainError> {
        let mut sorted = txs.keys().cloned().collect::<Vec<_>>();
        sorted.sort_by_key(|tx| {
            let is_mpn = if let TransactionData::UpdateContract { contract_id, .. } = &tx.tx.data {
                *contract_id == *MPN_CONTRACT_ID
            } else {
                false
            };
            (is_mpn, tx.tx.nonce)
        });
        if !check {
            return Ok(sorted);
        }
        let (_, result) = self.isolated(|chain| {
            let mut result = Vec::new();
            let mut block_sz = 0usize;
            let mut delta_sz = 0isize;
            for tx in sorted.into_iter() {
                if let Ok((ops, eff)) = chain.isolated(|chain| chain.apply_tx(&tx.tx, false)) {
                    let delta_diff = if let TxSideEffect::StateChange { state_change, .. } = eff {
                        state_change.state.size() as isize - state_change.prev_state.size() as isize
                    } else {
                        0
                    };
                    let block_diff = tx.tx.size();
                    if delta_sz + delta_diff <= chain.config.max_delta_size as isize
                        && block_sz + block_diff <= chain.config.max_block_size
                        && tx.tx.verify_signature()
                    {
                        delta_sz += delta_diff;
                        block_sz += block_diff;
                        chain.database.update(&ops)?;
                        result.push(tx);
                    }
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
                let fee_sum: Money = block.body[1..].iter().map(|t| t.fee).sum();

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
                        if amount != next_reward + fee_sum {
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

            let mut num_mpn_function_calls = 0;
            let mut num_mpn_contract_payments = 0;

            for tx in txs.iter() {
                // Count MPN updates
                if let TransactionData::UpdateContract {
                    contract_id,
                    updates,
                } = &tx.data
                {
                    if *contract_id == *MPN_CONTRACT_ID {
                        for update in updates.iter() {
                            match update {
                                ContractUpdate::Payment { .. } => {
                                    num_mpn_contract_payments += 1;
                                }
                                ContractUpdate::FunctionCall { .. } => {
                                    num_mpn_function_calls += 1;
                                }
                            }
                        }
                    }
                }

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

            if !is_genesis
                && (num_mpn_function_calls < self.config.mpn_num_function_calls
                    || num_mpn_contract_payments < self.config.mpn_num_contract_payments)
            {
                return Err(BlockchainError::InsufficientMpnUpdates);
            }

            if body_size > self.config.max_block_size {
                return Err(BlockchainError::BlockTooBig);
            }

            if state_size_delta > self.config.max_delta_size as isize {
                return Err(BlockchainError::StateDeltaTooBig);
            }

            chain.database.update(&[
                WriteOp::Put(keys::height(), (curr_height + 1).into()),
                WriteOp::Put(
                    keys::power(block.header.number),
                    (block.header.power() + self.get_power()?).into(),
                ),
            ])?;

            let rollback = chain.database.rollback()?;

            chain.database.update(&[
                WriteOp::Put(keys::rollback(block.header.number), rollback.into()),
                WriteOp::Put(
                    keys::header(block.header.number),
                    block.header.clone().into(),
                ),
                WriteOp::Put(keys::block(block.header.number), block.into()),
                WriteOp::Put(
                    keys::merkle(block.header.number),
                    block.merkle_tree().into(),
                ),
                WriteOp::Put(
                    keys::contract_updates(block.header.number),
                    state_updates.into(),
                ),
                if outdated_contracts.is_empty() {
                    WriteOp::Remove(keys::outdated())
                } else {
                    WriteOp::Put(keys::outdated(), outdated_contracts.clone().into())
                },
            ])?;

            Ok(())
        })?;

        self.database.update(&ops)?;
        Ok(())
    }
}

impl<K: KvStore> Blockchain for KvStoreChain<K> {
    fn get_header(&self, index: u64) -> Result<Header, BlockchainError> {
        if index >= self.get_height()? {
            return Err(BlockchainError::BlockNotFound);
        }
        Ok(match self.database.get(keys::header(index))? {
            Some(b) => b.try_into()?,
            None => {
                return Err(BlockchainError::Inconsistency);
            }
        })
    }

    fn get_block(&self, index: u64) -> Result<Block, BlockchainError> {
        if index >= self.get_height()? {
            return Err(BlockchainError::BlockNotFound);
        }
        Ok(match self.database.get(keys::block(index))? {
            Some(b) => b.try_into()?,
            None => {
                return Err(BlockchainError::Inconsistency);
            }
        })
    }

    fn rollback(&mut self) -> Result<(), BlockchainError> {
        let (ops, _) = self.isolated(|chain| {
            let height = chain.get_height()?;

            if height == 0 {
                return Err(BlockchainError::NoBlocksToRollback);
            }

            let rollback: Vec<WriteOp> = match chain.database.get(keys::rollback(height - 1))? {
                Some(b) => b.try_into()?,
                None => {
                    return Err(BlockchainError::Inconsistency);
                }
            };

            let mut outdated = chain.get_outdated_contracts()?;
            let changed_states = chain.get_changed_states(height - 1)?;

            for (cid, comp) in changed_states {
                if comp.prev_height == 0 {
                    zk::KvStoreStateManager::<ZkHasher>::delete_contract(&mut chain.database, cid)?;
                    outdated.retain(|&x| x != cid);
                    continue;
                }

                if !outdated.contains(&cid) {
                    let (ops, result) = chain.isolated(|fork| {
                        Ok(zk::KvStoreStateManager::<ZkHasher>::rollback_contract(
                            &mut fork.database,
                            cid,
                        )?)
                    })?;

                    if result != Some(comp.prev_state) && comp.prev_height > 0 {
                        outdated.push(cid);
                    } else {
                        chain.database.update(&ops)?;
                    }
                } else {
                    let local_compressed_state =
                        zk::KvStoreStateManager::<ZkHasher>::root(&chain.database, cid)?;
                    if local_compressed_state == comp.prev_state {
                        outdated.retain(|&x| x != cid);
                    }
                }
            }

            chain.database.update(&rollback)?;
            chain.database.update(&[
                if outdated.is_empty() {
                    WriteOp::Remove(keys::outdated())
                } else {
                    WriteOp::Put(keys::outdated(), outdated.clone().into())
                },
                WriteOp::Remove(keys::header(height - 1).into()),
                WriteOp::Remove(keys::block(height - 1).into()),
                WriteOp::Remove(keys::merkle(height - 1).into()),
                WriteOp::Remove(keys::contract_updates(height - 1).into()),
                WriteOp::Remove(keys::rollback(height - 1)),
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
            let state_height = zk::KvStoreStateManager::<ZkHasher>::height_of(&self.database, cid)?;
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
        Ok(self
            .database
            .get(keys::contract(&contract_id))?
            .map(|b| b.try_into())
            .ok_or(BlockchainError::ContractNotFound)??)
    }
    fn get_contract_account(
        &self,
        contract_id: ContractId,
    ) -> Result<ContractAccount, BlockchainError> {
        Ok(self
            .database
            .get(keys::contract_account(&contract_id))?
            .map(|b| b.try_into())
            .ok_or(BlockchainError::ContractNotFound)??)
    }

    fn get_account(&self, addr: Address) -> Result<Account, BlockchainError> {
        Ok(match self.database.get(keys::account(&addr))? {
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
            .get(keys::power(from - 1))?
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
                        self.config.minimum_pow_difficulty,
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
    fn get_headers(&self, since: u64, count: u64) -> Result<Vec<Header>, BlockchainError> {
        let mut blks: Vec<Header> = Vec::new();
        let until = std::cmp::min(self.get_height()?, since + count);
        for i in since..until {
            blks.push(self.get_header(i)?);
        }
        Ok(blks)
    }
    fn get_blocks(&self, since: u64, count: u64) -> Result<Vec<Block>, BlockchainError> {
        let mut blks: Vec<Block> = Vec::new();
        let until = std::cmp::min(self.get_height()?, since + count);
        for i in since..until {
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
        mempool: &HashMap<TransactionAndDelta, TransactionStats>,
        wallet: &Wallet,
        check: bool,
    ) -> Result<Option<BlockAndPatch>, BlockchainError> {
        let height = self.get_height()?;
        let outdated_contracts = self.get_outdated_contracts()?;

        if !outdated_contracts.is_empty() {
            return Err(BlockchainError::StatesOutdated);
        }

        let last_header = self.get_header(height - 1)?;
        let treasury_nonce = self.get_account(Address::Treasury)?.nonce;

        let tx_and_deltas = self.select_transactions(mempool, check)?;
        let fee_sum: Money = tx_and_deltas.iter().map(|t| t.tx.fee).sum();

        let mut txs = vec![Transaction {
            src: Address::Treasury,
            data: TransactionData::RegularSend {
                dst: wallet.get_address(),
                amount: self.next_reward()? + fee_sum,
            },
            nonce: treasury_nonce + 1,
            fee: 0,
            sig: Signature::Unsigned,
        }];

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

        match self.isolated(|chain| {
            chain.apply_block(&blk, false)?; // Check if everything is ok
            chain.update_states(&block_delta)?;

            Ok(())
        }) {
            Err(BlockchainError::InsufficientMpnUpdates) => Ok(None),
            Err(e) => Err(e),
            Ok(_) => Ok(Some(BlockAndPatch {
                block: blk,
                patch: block_delta,
            })),
        }
    }

    fn get_power(&self) -> Result<u128, BlockchainError> {
        let height = self.get_height()?;
        if height == 0 {
            Ok(0)
        } else {
            Ok(self
                .database
                .get(keys::power(height - 1))?
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
                        let (_, rollback_results) =
                            zk::KvStoreStateManager::<ZkHasher>::reset_contract(
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
                        zk::KvStoreStateManager::<ZkHasher>::update_contract(
                            &mut chain.database,
                            cid,
                            delta,
                        )?;
                    }
                };

                if zk::KvStoreStateManager::<ZkHasher>::root(&chain.database, cid)?
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

    fn cleanup_mempool(
        &self,
        mempool: &mut HashMap<TransactionAndDelta, TransactionStats>,
    ) -> Result<(), BlockchainError> {
        let mut sorted = mempool.keys().cloned().collect::<Vec<_>>();
        sorted.sort_by_key(|tx| tx.tx.nonce);
        self.isolated(|chain| {
            for tx in sorted.into_iter() {
                if chain.apply_tx(&tx.tx, false).is_err() {
                    mempool.remove(&tx);
                }
            }
            Ok(())
        })?;
        Ok(())
    }

    fn cleanup_contract_payment_mempool(
        &self,
        mempool: &mut HashMap<ContractPayment, TransactionStats>,
    ) -> Result<(), BlockchainError> {
        let mut sorted = mempool.keys().cloned().collect::<Vec<_>>();
        sorted.sort_by_key(|tx| tx.nonce);
        self.isolated(|chain| {
            for tx in sorted.into_iter() {
                if chain.apply_contract_payment(&tx, None).is_err() {
                    mempool.remove(&tx);
                }
            }
            Ok(())
        })?;
        Ok(())
    }

    fn validate_zero_transaction(
        &self,
        _tx: &zk::ZeroTransaction,
    ) -> Result<bool, BlockchainError> {
        Ok(true)
    }

    fn validate_contract_payment(&self, tx: &ContractPayment) -> Result<bool, BlockchainError> {
        Ok(self
            .isolated(|chain| Ok(chain.apply_contract_payment(tx, None).is_ok()))?
            .1)
    }

    fn validate_transaction(
        &self,
        tx_delta: &TransactionAndDelta,
    ) -> Result<bool, BlockchainError> {
        Ok(self
            .isolated(|chain| {
                // TODO: Also check for delta validity
                Ok(chain.apply_tx(&tx_delta.tx, false).is_ok())
            })?
            .1)
    }

    fn read_state(
        &self,
        contract_id: ContractId,
        locator: zk::ZkDataLocator,
    ) -> Result<zk::ZkScalar, BlockchainError> {
        Ok(zk::KvStoreStateManager::<ZkHasher>::get_data(
            &self.database,
            contract_id,
            &locator,
        )?)
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
                let away =
                    zk::KvStoreStateManager::<ZkHasher>::height_of(&self.database, cid)? - height;
                blockchain_patch.patches.insert(
                    cid,
                    if let Some(delta) =
                        zk::KvStoreStateManager::<ZkHasher>::delta_of(&self.database, cid, away)?
                    {
                        zk::ZkStatePatch::Delta(delta)
                    } else {
                        zk::ZkStatePatch::Full(zk::KvStoreStateManager::<ZkHasher>::get_full_state(
                            &self.database,
                            cid,
                        )?)
                    },
                );
            }
        }

        Ok(blockchain_patch)
    }

    fn config(&self) -> &BlockchainConfig {
        &self.config
    }
}

#[cfg(test)]
mod test;
