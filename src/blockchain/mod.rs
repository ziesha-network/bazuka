mod error;
pub use error::*;

use crate::consensus::pow::Difficulty;
use crate::core::{
    hash::Hash, Account, Address, Amount, Block, ChainSourcedTx, ContractAccount, ContractDeposit,
    ContractId, ContractUpdate, ContractWithdraw, Hasher, Header, Money, MpnAddress, MpnDeposit,
    MpnSourcedTx, MpnWithdraw, ProofOfWork, RegularSendEntry, Signature, Token, TokenId,
    TokenUpdate, Transaction, TransactionAndDelta, TransactionData, ZkHasher as CoreZkHasher,
};
use crate::db::{keys, KvStore, RamMirrorKvStore, WriteOp};
use crate::utils;
use crate::wallet::TxBuilder;
use crate::zk;
use crate::zk::ZkHasher;

use rayon::prelude::*;

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct Mempool {
    mpn_log4_account_capacity: u8,
    chain_sourced: HashMap<Address, HashMap<ChainSourcedTx, TransactionStats>>,
    rejected_chain_sourced: HashMap<ChainSourcedTx, TransactionStats>,
    mpn_sourced: HashMap<MpnAddress, HashMap<MpnSourcedTx, TransactionStats>>,
    rejected_mpn_sourced: HashMap<MpnSourcedTx, TransactionStats>,
}

impl Mempool {
    pub fn new(mpn_log4_account_capacity: u8) -> Self {
        Self {
            mpn_log4_account_capacity,
            chain_sourced: Default::default(),
            rejected_chain_sourced: Default::default(),
            mpn_sourced: Default::default(),
            rejected_mpn_sourced: Default::default(),
        }
    }
}

impl Mempool {
    pub fn refresh<B: Blockchain>(
        &mut self,
        blockchain: &B,
        local_ts: u32,
        max_time_alive: Option<u32>,
        max_time_remember: Option<u32>,
    ) -> Result<(), BlockchainError> {
        let mut mpn_accounts = HashMap::new();
        let mut chain_accounts = HashMap::new();
        let mut mpn_txs_to_remove = Vec::new();
        let mut chain_txs_to_remove = Vec::new();
        for (tx, _) in self.chain_sourced() {
            let acc = chain_accounts
                .entry(tx.sender())
                .or_insert(blockchain.get_account(tx.sender())?);
            if tx.nonce() <= acc.nonce {
                chain_txs_to_remove.push(tx.clone());
            }
        }
        for (tx, _) in self.mpn_sourced() {
            let acc = mpn_accounts.entry(tx.sender()).or_insert(
                blockchain
                    .get_mpn_account(tx.sender().account_index(self.mpn_log4_account_capacity))?,
            );
            if tx.nonce() < acc.nonce {
                mpn_txs_to_remove.push(tx.clone());
            }
        }
        for tx in chain_txs_to_remove {
            self.chain_sourced.entry(tx.sender()).and_modify(|all| {
                all.remove(&tx);
            });
        }
        for tx in mpn_txs_to_remove {
            self.mpn_sourced.entry(tx.sender()).and_modify(|all| {
                all.remove(&tx);
            });
        }
        if let Some(max_time_alive) = max_time_alive {
            let chain_sourced = self
                .chain_sourced()
                .map(|(tx, stats)| (tx.clone(), stats.clone()))
                .collect::<Vec<_>>();
            let mpn_sourced = self
                .mpn_sourced()
                .map(|(tx, stats)| (tx.clone(), stats.clone()))
                .collect::<Vec<_>>();
            for (tx, stats) in chain_sourced {
                if !stats.is_local && local_ts - stats.first_seen > max_time_alive {
                    self.expire_chain_sourced(&tx);
                }
            }
            for (tx, stats) in mpn_sourced {
                if !stats.is_local && local_ts - stats.first_seen > max_time_alive {
                    self.expire_mpn_sourced(&tx);
                }
            }
        }
        if let Some(max_time_remember) = max_time_remember {
            for (tx, stats) in self.rejected_chain_sourced.clone().into_iter() {
                if !stats.is_local && local_ts - stats.first_seen > max_time_remember {
                    self.rejected_chain_sourced.remove(&tx);
                }
            }
            for (tx, stats) in self.rejected_mpn_sourced.clone().into_iter() {
                if !stats.is_local && local_ts - stats.first_seen > max_time_remember {
                    self.rejected_mpn_sourced.remove(&tx);
                }
            }
        }
        Ok(())
    }
    pub fn chain_address_limit(&self, _addr: Address) -> usize {
        100
    }
    pub fn mpn_address_limit(&self, _addr: MpnAddress) -> usize {
        100
    }
    pub fn mpn_sourced_len(&self) -> usize {
        self.mpn_sourced.values().map(|c| c.len()).sum()
    }
    pub fn chain_sourced_len(&self) -> usize {
        self.chain_sourced.values().map(|c| c.len()).sum()
    }
    pub fn add_chain_sourced(&mut self, tx: ChainSourcedTx, is_local: bool, now: u32) {
        if is_local {
            self.rejected_chain_sourced.remove(&tx);
        }
        if !self.rejected_chain_sourced.contains_key(&tx) {
            if self
                .chain_sourced
                .get(&tx.sender())
                .map(|all| all.contains_key(&tx))
                .unwrap_or_default()
            {
                return;
            }

            let limit = self.chain_address_limit(tx.sender());
            let all = self.chain_sourced.entry(tx.sender().clone()).or_default();
            if is_local || all.len() < limit {
                if tx.verify_signature() {
                    all.insert(tx.clone(), TransactionStats::new(is_local, now));
                }
            }
        }
    }
    pub fn add_mpn_sourced(&mut self, tx: MpnSourcedTx, is_local: bool, now: u32) {
        if is_local {
            self.rejected_mpn_sourced.remove(&tx);
        }
        if !self.rejected_mpn_sourced.contains_key(&tx) {
            if self
                .mpn_sourced
                .get(&tx.sender())
                .map(|all| all.contains_key(&tx))
                .unwrap_or_default()
            {
                return;
            }

            let limit = self.mpn_address_limit(tx.sender());
            let all = self.mpn_sourced.entry(tx.sender().clone()).or_default();
            if tx.verify_signature() {
                if is_local || all.len() < limit {
                    all.insert(tx.clone(), TransactionStats::new(is_local, now));
                }
            }
        }
    }
    pub fn reject_chain_sourced(&mut self, tx: &ChainSourcedTx, e: BlockchainError) {
        if let Some(all) = self.chain_sourced.get_mut(&tx.sender()) {
            if let Some(stats) = all.remove(tx) {
                // Nonce issues do not need rejection
                if let BlockchainError::InvalidTransactionNonce = e {
                    self.rejected_chain_sourced.insert(tx.clone(), stats);
                }
            }
        }
        if self
            .chain_sourced
            .get(&tx.sender())
            .map(|all| all.is_empty())
            .unwrap_or(true)
        {
            self.chain_sourced.remove(&tx.sender());
        }
    }
    pub fn reject_mpn_sourced(&mut self, tx: &MpnSourcedTx, e: BlockchainError) {
        if let Some(all) = self.mpn_sourced.get_mut(&tx.sender()) {
            if let Some(stats) = all.remove(tx) {
                // Nonce issues do not need rejection
                if let BlockchainError::InvalidTransactionNonce = e {
                    self.rejected_mpn_sourced.insert(tx.clone(), stats);
                }
            }
        }
        if self
            .mpn_sourced
            .get(&tx.sender())
            .clone()
            .map(|all| all.is_empty())
            .unwrap_or(true)
        {
            self.mpn_sourced.remove(&tx.sender());
        }
    }
    pub fn expire_chain_sourced(&mut self, _tx: &ChainSourcedTx) {
        //self.reject_chain_sourced(tx);
    }
    pub fn expire_mpn_sourced(&mut self, _tx: &MpnSourcedTx) {
        //self.reject_mpn_sourced(tx);
    }
    pub fn chain_sourced(&self) -> impl Iterator<Item = (&ChainSourcedTx, &TransactionStats)> {
        self.chain_sourced.iter().map(|(_, c)| c.iter()).flatten()
    }
    pub fn mpn_sourced(&self) -> impl Iterator<Item = (&MpnSourcedTx, &TransactionStats)> {
        self.mpn_sourced.iter().map(|(_, c)| c.iter()).flatten()
    }
}

#[derive(Clone)]
pub struct BlockchainConfig {
    pub limited_miners: Option<HashSet<Address>>,
    pub genesis: BlockAndPatch,
    pub reward_ratio: u64,
    pub max_block_size: usize,
    pub max_delta_count: usize,
    pub block_time: u32,
    pub difficulty_window: u64,
    pub difficulty_lag: u64,
    pub difficulty_cut: u64,
    pub pow_base_key: &'static [u8],
    pub pow_key_change_delay: u64,
    pub pow_key_change_interval: u64,
    pub median_timestamp_count: u64,
    pub ziesha_token_id: TokenId,
    pub mpn_contract_id: ContractId,
    pub mpn_num_function_calls: usize,
    pub mpn_num_contract_deposits: usize,
    pub mpn_num_contract_withdraws: usize,
    pub mpn_log4_account_capacity: u8,
    pub mpn_proving_time: u32,
    pub minimum_pow_difficulty: Difficulty,
    pub testnet_height_limit: Option<u64>,
    pub max_memo_length: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransactionValidity {
    Unknown,
    Invalid,
    Valid,
}

#[derive(Debug, Clone)]
pub struct TransactionStats {
    pub first_seen: u32,
    pub validity: TransactionValidity,
    pub is_local: bool,
}

impl TransactionStats {
    pub fn new(is_local: bool, first_seen: u32) -> Self {
        Self {
            first_seen,
            validity: TransactionValidity::Unknown,
            is_local,
        }
    }
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

// A proof that you are the validator for this block
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidatorProof {}

pub trait Blockchain {
    fn validator_set(&self) -> Result<Vec<Address>, BlockchainError>;
    fn is_validator(
        &self,
        timestamp: u32,
        addr: Address,
        proof: ValidatorProof,
    ) -> Result<bool, BlockchainError>;
    fn validator_status(
        &self,
        timestamp: u32,
        wallet: &TxBuilder,
    ) -> Result<Option<ValidatorProof>, BlockchainError>;

    fn currency_in_circulation(&self) -> Result<Amount, BlockchainError>;

    fn config(&self) -> &BlockchainConfig;

    fn cleanup_chain_mempool(&self, mempool: &mut Mempool) -> Result<(), BlockchainError>;
    fn cleanup_mpn_mempool(
        &self,
        mempool: &mut Mempool,
        capacity: usize,
    ) -> Result<(), BlockchainError>;

    fn db_checksum(&self) -> Result<String, BlockchainError>;

    fn get_token(&self, token_id: TokenId) -> Result<Option<Token>, BlockchainError>;

    fn get_balance(&self, addr: Address, token_id: TokenId) -> Result<Amount, BlockchainError>;
    fn get_contract_balance(
        &self,
        contract_id: ContractId,
        token_id: TokenId,
    ) -> Result<Amount, BlockchainError>;
    fn get_account(&self, addr: Address) -> Result<Account, BlockchainError>;
    fn get_mpn_account(&self, index: u64) -> Result<zk::MpnAccount, BlockchainError>;
    fn get_mpn_accounts(
        &self,
        page: usize,
        page_size: usize,
    ) -> Result<Vec<(u64, zk::MpnAccount)>, BlockchainError>;

    fn get_contract_account(
        &self,
        contract_id: ContractId,
    ) -> Result<ContractAccount, BlockchainError>;
    fn read_state(
        &self,
        contract_id: ContractId,
        locator: zk::ZkDataLocator,
    ) -> Result<zk::ZkScalar, BlockchainError>;
    fn next_reward(&self) -> Result<Amount, BlockchainError>;
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
        mempool: &[TransactionAndDelta],
        wallet: &TxBuilder,
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
    pub fn db(&self) -> &K {
        &self.database
    }
    pub fn new(database: K, config: BlockchainConfig) -> Result<KvStoreChain<K>, BlockchainError> {
        let mut chain = KvStoreChain::<K> {
            database,
            config: config.clone(),
        };
        if chain.get_height()? == 0 {
            chain.apply_block(&config.genesis.block, true)?;
            chain.update_states(&config.genesis.patch)?;
        } else if config.genesis.block != chain.get_block(0)? {
            return Err(BlockchainError::DifferentGenesis);
        }

        Ok(chain)
    }

    pub fn fork_on_ram(&self) -> KvStoreChain<RamMirrorKvStore<'_, K>> {
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

    fn next_difficulty(&self) -> Result<Difficulty, BlockchainError> {
        let to = self
            .get_height()?
            .saturating_sub(self.config.difficulty_lag);
        let from = to.saturating_sub(self.config.difficulty_window);
        let headers = (from..to)
            .map(|i| self.get_header(i))
            .collect::<Result<Vec<_>, _>>()?;
        let mut timestamps = headers
            .iter()
            .map(|h| h.proof_of_work.timestamp)
            .collect::<Vec<_>>();
        if timestamps.len() < 2 {
            return Ok(self.config.minimum_pow_difficulty);
        }
        timestamps.sort_unstable();
        let final_size = self.config.difficulty_window - 2 * self.config.difficulty_cut;
        let (begin, end) = if timestamps.len() as u64 > final_size {
            let begin = (timestamps.len() as u64 - final_size + 1) / 2;
            let end = begin + final_size - 1;
            (begin as usize, end as usize)
        } else {
            (0, timestamps.len() - 1)
        };
        let time_delta = (timestamps[end] - timestamps[begin])
            .saturating_sub((end - begin) as u32 * self.config.mpn_proving_time as u32);
        if time_delta == 0 {
            return Ok(self.config.minimum_pow_difficulty);
        }
        let power_delta =
            self.get_power_at(from + end as u64)? - self.get_power_at(from + begin as u64)?;
        Ok(std::cmp::max(
            Difficulty::from_power(
                power_delta * (self.config.block_time as u128) / (time_delta as u128),
            ),
            self.config.minimum_pow_difficulty,
        ))
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
            return Ok(zk::ZkCompressedState::empty::<CoreZkHasher>(state_model));
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

    fn apply_deposit(&mut self, deposit: &ContractDeposit) -> Result<(), BlockchainError> {
        let (ops, _) = self.isolated(|chain| {
            if !deposit.verify_signature() {
                return Err(BlockchainError::InvalidContractPaymentSignature);
            }

            let mut addr_account = chain.get_account(deposit.src.clone())?;
            if deposit.nonce != addr_account.nonce + 1 {
                return Err(BlockchainError::InvalidTransactionNonce);
            }
            addr_account.nonce += 1;
            chain.database.update(&[WriteOp::Put(
                keys::account(&deposit.src.clone()),
                addr_account.into(),
            )])?;

            if deposit.amount.token_id == deposit.fee.token_id {
                let mut addr_balance =
                    chain.get_balance(deposit.src.clone(), deposit.amount.token_id)?;

                if addr_balance < deposit.amount.amount + deposit.fee.amount {
                    return Err(BlockchainError::BalanceInsufficient);
                }
                addr_balance -= deposit.amount.amount + deposit.fee.amount;
                chain.database.update(&[WriteOp::Put(
                    keys::account_balance(&deposit.src, deposit.amount.token_id),
                    addr_balance.into(),
                )])?;
            } else {
                let mut addr_balance =
                    chain.get_balance(deposit.src.clone(), deposit.amount.token_id)?;
                let mut addr_fee_balance =
                    chain.get_balance(deposit.src.clone(), deposit.fee.token_id)?;

                if addr_balance < deposit.amount.amount {
                    return Err(BlockchainError::BalanceInsufficient);
                }
                if addr_fee_balance < deposit.fee.amount {
                    return Err(BlockchainError::BalanceInsufficient);
                }
                addr_fee_balance -= deposit.fee.amount;
                addr_balance -= deposit.amount.amount;
                chain.database.update(&[WriteOp::Put(
                    keys::account_balance(&deposit.src, deposit.amount.token_id),
                    addr_balance.into(),
                )])?;
                chain.database.update(&[WriteOp::Put(
                    keys::account_balance(&deposit.src, deposit.fee.token_id),
                    addr_fee_balance.into(),
                )])?;
            }

            let mut contract_balance =
                chain.get_contract_balance(deposit.contract_id, deposit.amount.token_id)?;
            contract_balance += deposit.amount.amount;
            chain.database.update(&[WriteOp::Put(
                keys::contract_balance(&deposit.contract_id, deposit.amount.token_id),
                contract_balance.into(),
            )])?;
            Ok(())
        })?;
        self.database.update(&ops)?;
        Ok(())
    }

    fn apply_mpn_deposit(&mut self, deposit: &MpnDeposit) -> Result<(), BlockchainError> {
        let (ops, _) = self.isolated(|chain| {
            chain.apply_deposit(&deposit.payment)?;
            let dst = chain
                .get_mpn_account(deposit.zk_address_index(self.config.mpn_log4_account_capacity))?;

            let calldata = CoreZkHasher::hash(&[
                deposit.zk_address.decompress().0,
                deposit.zk_address.decompress().1,
            ]);
            if deposit.payment.calldata != calldata
                || deposit.zk_token_index >= 64
                || deposit.zk_address_index(self.config.mpn_log4_account_capacity) > 0x3FFFFFFF
                || !deposit.zk_address.is_on_curve()
                || (dst.address.is_on_curve() && dst.address != deposit.zk_address.decompress())
            {
                return Err(BlockchainError::InvalidMpnTransaction);
            }
            if let Some(money) = dst.tokens.get(&deposit.zk_token_index) {
                if money.token_id != deposit.payment.amount.token_id {
                    return Err(BlockchainError::InvalidMpnTransaction);
                }
            }
            // TODO: Check overflow
            let mut size_diff = 0;
            let mut new_acc = zk::MpnAccount {
                address: deposit.zk_address.0.decompress(),
                nonce: dst.nonce,
                tokens: dst.tokens,
            };
            new_acc
                .tokens
                .entry(deposit.zk_token_index)
                .or_insert_with(|| Money::new(deposit.payment.amount.token_id, 0))
                .amount += deposit.payment.amount.amount;

            zk::KvStoreStateManager::<CoreZkHasher>::set_mpn_account(
                &mut chain.database,
                chain.config.mpn_contract_id,
                deposit.zk_address_index(self.config.mpn_log4_account_capacity),
                new_acc,
                &mut size_diff,
            )?;
            Ok(())
        })?;
        self.database.update(&ops)?;
        Ok(())
    }

    fn apply_mpn_withdraw(&mut self, withdraw: &MpnWithdraw) -> Result<(), BlockchainError> {
        let (ops, _) = self.isolated(|chain| {
            chain.apply_withdraw(&withdraw.payment)?;
            let src = chain.get_mpn_account(
                withdraw.zk_address_index(self.config.mpn_log4_account_capacity),
            )?;
            if withdraw.zk_nonce != src.nonce {
                return Err(BlockchainError::InvalidTransactionNonce);
            }
            let calldata = CoreZkHasher::hash(&[
                withdraw.zk_address.decompress().0,
                withdraw.zk_address.decompress().1,
                zk::ZkScalar::from(withdraw.zk_nonce),
                withdraw.zk_sig.r.0,
                withdraw.zk_sig.r.1,
                withdraw.zk_sig.s,
            ]);
            if withdraw.zk_address_index(self.config.mpn_log4_account_capacity) > 0x3FFFFFFF
                || withdraw.zk_token_index >= 64
                || withdraw.payment.calldata != calldata
                || !withdraw.verify_signature::<CoreZkHasher>()
            {
                return Err(BlockchainError::InvalidMpnTransaction);
            }

            let mut size_diff = 0;
            let mut new_acc = zk::MpnAccount {
                address: src.address,
                tokens: src.tokens.clone(),
                nonce: src.nonce + 1,
            };
            if let Some(money) = new_acc.tokens.get_mut(&withdraw.zk_token_index) {
                if money.token_id == withdraw.payment.amount.token_id
                    && money.amount >= withdraw.payment.amount.amount
                {
                    money.amount -= withdraw.payment.amount.amount;
                } else {
                    return Err(BlockchainError::InvalidMpnTransaction);
                }
            } else {
                return Err(BlockchainError::InvalidMpnTransaction);
            }
            if let Some(money) = new_acc.tokens.get_mut(&withdraw.zk_fee_token_index) {
                if money.token_id == withdraw.payment.fee.token_id
                    && money.amount >= withdraw.payment.fee.amount
                {
                    money.amount -= withdraw.payment.fee.amount;
                } else {
                    return Err(BlockchainError::InvalidMpnTransaction);
                }
            } else {
                return Err(BlockchainError::InvalidMpnTransaction);
            }

            zk::KvStoreStateManager::<CoreZkHasher>::set_mpn_account(
                &mut chain.database,
                chain.config.mpn_contract_id,
                withdraw.zk_address_index(self.config.mpn_log4_account_capacity),
                new_acc,
                &mut size_diff,
            )?;
            Ok(())
        })?;
        self.database.update(&ops)?;
        Ok(())
    }

    fn apply_withdraw(&mut self, withdraw: &ContractWithdraw) -> Result<(), BlockchainError> {
        let (ops, _) = self.isolated(|chain| {
            if withdraw.amount.token_id == withdraw.fee.token_id {
                let mut contract_balance =
                    chain.get_contract_balance(withdraw.contract_id, withdraw.amount.token_id)?;

                if contract_balance < withdraw.amount.amount + withdraw.fee.amount {
                    return Err(BlockchainError::ContractBalanceInsufficient);
                }
                contract_balance -= withdraw.amount.amount + withdraw.fee.amount;
                chain.database.update(&[WriteOp::Put(
                    keys::contract_balance(&withdraw.contract_id, withdraw.amount.token_id),
                    contract_balance.into(),
                )])?;
            } else {
                let mut contract_balance =
                    chain.get_contract_balance(withdraw.contract_id, withdraw.amount.token_id)?;
                let mut contract_fee_balance =
                    chain.get_contract_balance(withdraw.contract_id, withdraw.fee.token_id)?;

                if contract_balance < withdraw.amount.amount {
                    return Err(BlockchainError::ContractBalanceInsufficient);
                }
                if contract_fee_balance < withdraw.fee.amount {
                    return Err(BlockchainError::ContractBalanceInsufficient);
                }
                contract_fee_balance -= withdraw.fee.amount;
                contract_balance -= withdraw.amount.amount;
                chain.database.update(&[WriteOp::Put(
                    keys::contract_balance(&withdraw.contract_id, withdraw.amount.token_id),
                    contract_balance.into(),
                )])?;
                chain.database.update(&[WriteOp::Put(
                    keys::contract_balance(&withdraw.contract_id, withdraw.fee.token_id),
                    contract_fee_balance.into(),
                )])?;
            }

            let mut addr_balance =
                chain.get_balance(withdraw.dst.clone(), withdraw.amount.token_id)?;
            addr_balance += withdraw.amount.amount;
            chain.database.update(&[WriteOp::Put(
                keys::account_balance(&withdraw.dst, withdraw.amount.token_id),
                addr_balance.into(),
            )])?;

            Ok(())
        })?;
        self.database.update(&ops)?;
        Ok(())
    }

    // WARN: Will not check sig!
    fn apply_zero_tx(&mut self, tx: &zk::MpnTransaction) -> Result<(), BlockchainError> {
        let (ops, _) = self.isolated(|chain| {
            let tx_src_index = tx.src_index(self.config.mpn_log4_account_capacity);
            let tx_dst_index = tx.dst_index(self.config.mpn_log4_account_capacity);
            if tx_src_index == tx_dst_index {
                let src = chain.get_mpn_account(tx_src_index)?;
                if !tx.src_pub_key.is_on_curve()
                    || !tx.dst_pub_key.is_on_curve()
                    || src.address != tx.src_pub_key.decompress()
                    || tx.nonce != src.nonce
                {
                    return Err(BlockchainError::InvalidMpnTransaction);
                }

                let mut size_diff = 0;
                let mut new_src_acc = zk::MpnAccount {
                    address: src.address,
                    tokens: src.tokens.clone(),
                    nonce: src.nonce + 1,
                };
                if let Some(src_money) = new_src_acc.tokens.get_mut(&tx.src_token_index) {
                    if src_money.token_id != tx.amount.token_id {
                        return Err(BlockchainError::InvalidMpnTransaction);
                    }
                } else {
                    return Err(BlockchainError::InvalidMpnTransaction);
                }
                if let Some(money) = new_src_acc.tokens.get_mut(&tx.src_fee_token_index) {
                    if money.token_id == tx.fee.token_id && money.amount >= tx.fee.amount {
                        money.amount -= tx.fee.amount;
                    } else {
                        return Err(BlockchainError::InvalidMpnTransaction);
                    }
                } else {
                    return Err(BlockchainError::InvalidMpnTransaction);
                }

                zk::KvStoreStateManager::<CoreZkHasher>::set_mpn_account(
                    &mut chain.database,
                    chain.config.mpn_contract_id,
                    tx_src_index,
                    new_src_acc,
                    &mut size_diff,
                )?;
            } else {
                let src = chain.get_mpn_account(tx_src_index)?;
                let dst = chain.get_mpn_account(tx_dst_index)?;
                if !tx.src_pub_key.is_on_curve()
                    || !tx.dst_pub_key.is_on_curve()
                    || src.address != tx.src_pub_key.decompress()
                    || (dst.address.is_on_curve() && dst.address != tx.dst_pub_key.decompress())
                    || tx.nonce != src.nonce
                {
                    return Err(BlockchainError::InvalidMpnTransaction);
                }

                let mut size_diff = 0;
                let mut new_src_acc = zk::MpnAccount {
                    address: src.address,
                    tokens: src.tokens.clone(),
                    nonce: src.nonce + 1,
                };
                let mut new_dst_acc = zk::MpnAccount {
                    address: tx.dst_pub_key.0.decompress(),
                    tokens: dst.tokens.clone(),
                    nonce: dst.nonce,
                };
                let src_token_balance_mut = new_src_acc.tokens.get_mut(&tx.src_token_index);
                if let Some(src_money) = src_token_balance_mut {
                    let dst_money = new_dst_acc
                        .tokens
                        .entry(tx.dst_token_index)
                        .or_insert_with(|| Money::new(src_money.token_id, 0));
                    if src_money.token_id == dst_money.token_id
                        && src_money.token_id == tx.amount.token_id
                    {
                        if src_money.amount >= tx.amount.amount {
                            src_money.amount -= tx.amount.amount;
                            dst_money.amount += tx.amount.amount;
                        } else {
                            return Err(BlockchainError::InvalidMpnTransaction);
                        }
                    } else {
                        return Err(BlockchainError::InvalidMpnTransaction);
                    }
                } else {
                    return Err(BlockchainError::InvalidMpnTransaction);
                }
                if let Some(money) = new_src_acc.tokens.get_mut(&tx.src_fee_token_index) {
                    if money.token_id == tx.fee.token_id && money.amount >= tx.fee.amount {
                        money.amount -= tx.fee.amount;
                    } else {
                        return Err(BlockchainError::InvalidMpnTransaction);
                    }
                } else {
                    return Err(BlockchainError::InvalidMpnTransaction);
                }

                zk::KvStoreStateManager::<CoreZkHasher>::set_mpn_account(
                    &mut chain.database,
                    chain.config.mpn_contract_id,
                    tx_src_index,
                    new_src_acc,
                    &mut size_diff,
                )?;
                zk::KvStoreStateManager::<CoreZkHasher>::set_mpn_account(
                    &mut chain.database,
                    chain.config.mpn_contract_id,
                    tx_dst_index,
                    new_dst_acc,
                    &mut size_diff,
                )?;
            }
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

            if tx.src == None && !allow_treasury {
                return Err(BlockchainError::IllegalTreasuryAccess);
            }

            if tx.fee.token_id != TokenId::Ziesha {
                return Err(BlockchainError::OnlyZieshaFeesAccepted);
            }

            if tx.memo.len() > self.config.max_memo_length {
                return Err(BlockchainError::MemoTooLong);
            }

            let tx_src = tx.src.clone().unwrap_or_default(); // Default is treasury account!

            let mut acc_src = chain.get_account(tx_src.clone())?;
            let mut acc_bal = chain.get_balance(tx_src.clone(), tx.fee.token_id)?;

            if tx.nonce != acc_src.nonce + 1 {
                return Err(BlockchainError::InvalidTransactionNonce);
            }

            if acc_bal < tx.fee.amount {
                return Err(BlockchainError::BalanceInsufficient);
            }

            acc_bal -= tx.fee.amount;
            acc_src.nonce += 1;

            chain
                .database
                .update(&[WriteOp::Put(keys::account(&tx_src), acc_src.into())])?;
            chain.database.update(&[WriteOp::Put(
                keys::account_balance(&tx_src, tx.fee.token_id),
                acc_bal.into(),
            )])?;

            match &tx.data {
                TransactionData::CreateToken { token } => {
                    let token_id = {
                        let tid = TokenId::new(tx);
                        if tid == self.config.ziesha_token_id {
                            TokenId::Ziesha
                        } else {
                            tid
                        }
                    };
                    if chain.get_token(token_id)?.is_some() {
                        return Err(BlockchainError::TokenAlreadyExists);
                    } else {
                        if !token.validate() {
                            return Err(BlockchainError::TokenBadNameSymbol);
                        }
                        chain.database.update(&[WriteOp::Put(
                            keys::account_balance(&tx_src, token_id),
                            token.supply.into(),
                        )])?;
                        chain
                            .database
                            .update(&[WriteOp::Put(keys::token(&token_id), token.into())])?;
                    }
                }
                TransactionData::UpdateToken { token_id, update } => {
                    if let Some(mut token) = chain.get_token(*token_id)? {
                        if let Some(minter) = token.minter.clone() {
                            if minter == tx_src {
                                match update {
                                    TokenUpdate::Mint { amount } => {
                                        let mut bal =
                                            chain.get_balance(tx_src.clone(), *token_id)?;
                                        if bal + *amount < bal
                                            || token.supply + *amount < token.supply
                                        {
                                            return Err(BlockchainError::TokenSupplyOverflow);
                                        }
                                        bal += *amount;
                                        token.supply += *amount;
                                        chain.database.update(&[WriteOp::Put(
                                            keys::token(token_id),
                                            (&token).into(),
                                        )])?;
                                        chain.database.update(&[WriteOp::Put(
                                            keys::account_balance(&tx_src, *token_id),
                                            bal.into(),
                                        )])?;
                                    }
                                    TokenUpdate::ChangeMinter { minter } => {
                                        token.minter = Some(minter.clone());
                                        chain.database.update(&[WriteOp::Put(
                                            keys::token(token_id),
                                            (&token).into(),
                                        )])?;
                                    }
                                }
                            } else {
                                return Err(BlockchainError::TokenUpdatePermissionDenied);
                            }
                        } else {
                            return Err(BlockchainError::TokenNotUpdatable);
                        }
                    } else {
                        return Err(BlockchainError::TokenNotFound);
                    }
                }
                TransactionData::RegularSend { entries } => {
                    for entry in entries {
                        if entry.dst != tx_src {
                            let mut src_bal =
                                chain.get_balance(tx_src.clone(), entry.amount.token_id)?;

                            if src_bal < entry.amount.amount {
                                return Err(BlockchainError::BalanceInsufficient);
                            }
                            src_bal -= entry.amount.amount;
                            chain.database.update(&[WriteOp::Put(
                                keys::account_balance(&tx_src, entry.amount.token_id),
                                src_bal.into(),
                            )])?;

                            let mut dst_bal =
                                chain.get_balance(entry.dst.clone(), entry.amount.token_id)?;
                            dst_bal += entry.amount.amount;

                            chain.database.update(&[WriteOp::Put(
                                keys::account_balance(&entry.dst, entry.amount.token_id),
                                dst_bal.into(),
                            )])?;
                        }
                    }
                }
                TransactionData::CreateContract { contract } => {
                    if !contract.state_model.is_valid::<CoreZkHasher>() {
                        return Err(BlockchainError::InvalidStateModel);
                    }
                    let contract_id = ContractId::new(tx);
                    chain.database.update(&[WriteOp::Put(
                        keys::contract(&contract_id),
                        contract.clone().into(),
                    )])?;
                    let compressed_empty =
                        zk::ZkCompressedState::empty::<CoreZkHasher>(contract.state_model.clone());
                    chain.database.update(&[WriteOp::Put(
                        keys::contract_account(&contract_id),
                        ContractAccount {
                            compressed_state: contract.initial_state,
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
                    let mut executor_fees = Vec::new();

                    let prev_account = chain.get_contract_account(*contract_id)?;

                    // Increase height
                    let mut cont_account = chain.get_contract_account(*contract_id)?;
                    cont_account.height += 1;
                    chain.database.update(&[WriteOp::Put(
                        keys::contract_account(contract_id),
                        cont_account.into(),
                    )])?;

                    for update in updates {
                        let (circuit, aux_data, next_state, proof) = match update {
                            ContractUpdate::Deposit {
                                deposit_circuit_id,
                                deposits,
                                next_state,
                                proof,
                            } => {
                                let deposit_func = contract
                                    .deposit_functions
                                    .get(*deposit_circuit_id as usize)
                                    .ok_or(BlockchainError::ContractFunctionNotFound)?;
                                let circuit = &deposit_func.verifier_key;
                                let state_model = zk::ZkStateModel::List {
                                    item_type: Box::new(zk::ZkStateModel::Struct {
                                        field_types: vec![
                                            zk::ZkStateModel::Scalar, // Enabled
                                            zk::ZkStateModel::Scalar, // Token-id
                                            zk::ZkStateModel::Scalar, // Amount
                                            zk::ZkStateModel::Scalar, // Calldata
                                        ],
                                    }),
                                    log4_size: deposit_func.log4_payment_capacity,
                                };
                                let mut state_builder =
                                    zk::ZkStateBuilder::<CoreZkHasher>::new(state_model);
                                for (i, deposit) in deposits.iter().enumerate() {
                                    if deposit.contract_id != *contract_id
                                        || deposit.deposit_circuit_id != *deposit_circuit_id
                                    {
                                        return Err(
                                            BlockchainError::DepositWithdrawPassedToWrongFunction,
                                        );
                                    }
                                    if deposit.src.clone() == tx_src {
                                        return Err(BlockchainError::CannotExecuteOwnPayments);
                                    }
                                    executor_fees.push(deposit.fee);
                                    state_builder.batch_set(&zk::ZkDeltaPairs(
                                        [
                                            (
                                                zk::ZkDataLocator(vec![i as u64, 0]),
                                                Some(zk::ZkScalar::from(1)),
                                            ),
                                            (
                                                zk::ZkDataLocator(vec![i as u64, 1]),
                                                Some(deposit.amount.token_id.into()),
                                            ),
                                            (
                                                zk::ZkDataLocator(vec![i as u64, 2]),
                                                Some(zk::ZkScalar::from(deposit.amount.amount)),
                                            ),
                                            (
                                                zk::ZkDataLocator(vec![i as u64, 3]),
                                                Some(deposit.calldata),
                                            ),
                                        ]
                                        .into(),
                                    ))?;

                                    chain.apply_deposit(deposit)?;
                                }
                                let aux_data = state_builder.compress()?;
                                (circuit, aux_data, next_state, proof)
                            }
                            ContractUpdate::Withdraw {
                                withdraw_circuit_id,
                                withdraws,
                                next_state,
                                proof,
                            } => {
                                let withdraw_func = contract
                                    .withdraw_functions
                                    .get(*withdraw_circuit_id as usize)
                                    .ok_or(BlockchainError::ContractFunctionNotFound)?;
                                let circuit = &withdraw_func.verifier_key;
                                let state_model = zk::ZkStateModel::List {
                                    item_type: Box::new(zk::ZkStateModel::Struct {
                                        field_types: vec![
                                            zk::ZkStateModel::Scalar, // Enabled
                                            zk::ZkStateModel::Scalar, // Amount token-id
                                            zk::ZkStateModel::Scalar, // Amount
                                            zk::ZkStateModel::Scalar, // Fee token-id
                                            zk::ZkStateModel::Scalar, // Fee
                                            zk::ZkStateModel::Scalar, // Fingerprint
                                            zk::ZkStateModel::Scalar, // Calldata
                                        ],
                                    }),
                                    log4_size: withdraw_func.log4_payment_capacity,
                                };
                                let mut state_builder =
                                    zk::ZkStateBuilder::<CoreZkHasher>::new(state_model);
                                for (i, withdraw) in withdraws.iter().enumerate() {
                                    if withdraw.contract_id != *contract_id
                                        || withdraw.withdraw_circuit_id != *withdraw_circuit_id
                                    {
                                        return Err(
                                            BlockchainError::DepositWithdrawPassedToWrongFunction,
                                        );
                                    }
                                    let fingerprint = withdraw.fingerprint();
                                    if withdraw.dst.clone() == tx_src {
                                        return Err(BlockchainError::CannotExecuteOwnPayments);
                                    }
                                    executor_fees.push(withdraw.fee);
                                    state_builder.batch_set(&zk::ZkDeltaPairs(
                                        [
                                            (
                                                zk::ZkDataLocator(vec![i as u64, 0]),
                                                Some(zk::ZkScalar::from(1)),
                                            ),
                                            (
                                                zk::ZkDataLocator(vec![i as u64, 1]),
                                                Some(withdraw.amount.token_id.into()),
                                            ),
                                            (
                                                zk::ZkDataLocator(vec![i as u64, 2]),
                                                Some(zk::ZkScalar::from(withdraw.amount.amount)),
                                            ),
                                            (
                                                zk::ZkDataLocator(vec![i as u64, 3]),
                                                Some(withdraw.fee.token_id.into()),
                                            ),
                                            (
                                                zk::ZkDataLocator(vec![i as u64, 4]),
                                                Some(zk::ZkScalar::from(withdraw.fee.amount)),
                                            ),
                                            (
                                                zk::ZkDataLocator(vec![i as u64, 5]),
                                                Some(fingerprint),
                                            ),
                                            (
                                                zk::ZkDataLocator(vec![i as u64, 6]),
                                                Some(withdraw.calldata),
                                            ),
                                        ]
                                        .into(),
                                    ))?;

                                    chain.apply_withdraw(withdraw)?;
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
                                executor_fees.push(*fee);

                                let mut cont_balance =
                                    chain.get_contract_balance(*contract_id, fee.token_id)?;
                                if cont_balance < fee.amount {
                                    return Err(BlockchainError::BalanceInsufficient);
                                }
                                cont_balance -= fee.amount;
                                chain.database.update(&[WriteOp::Put(
                                    keys::contract_balance(contract_id, fee.token_id),
                                    cont_balance.into(),
                                )])?;

                                let func = contract
                                    .functions
                                    .get(*function_id as usize)
                                    .ok_or(BlockchainError::ContractFunctionNotFound)?;
                                let circuit = &func.verifier_key;
                                let mut state_builder = zk::ZkStateBuilder::<CoreZkHasher>::new(
                                    zk::ZkStateModel::Struct {
                                        field_types: vec![
                                            zk::ZkStateModel::Scalar, // Token-id
                                            zk::ZkStateModel::Scalar, // Total fee
                                        ],
                                    },
                                );
                                state_builder.batch_set(&zk::ZkDeltaPairs(
                                    [
                                        (zk::ZkDataLocator(vec![0]), Some(fee.token_id.into())),
                                        (
                                            zk::ZkDataLocator(vec![1]),
                                            Some(zk::ZkScalar::from(fee.amount)),
                                        ),
                                    ]
                                    .into(),
                                ))?;
                                let aux_data = state_builder.compress()?;
                                (circuit, aux_data, next_state, proof)
                            }
                        };

                        let mut cont_account = chain.get_contract_account(*contract_id)?;
                        if !zk::check_proof(
                            circuit,
                            prev_account.height,
                            &cont_account.compressed_state,
                            &aux_data,
                            next_state,
                            proof,
                        ) {
                            return Err(BlockchainError::IncorrectZkProof);
                        }
                        cont_account.compressed_state = *next_state;
                        chain.database.update(&[WriteOp::Put(
                            keys::contract_account(contract_id),
                            cont_account.into(),
                        )])?;
                    }

                    for fee in executor_fees {
                        let mut acc_bal = chain.get_balance(tx_src.clone(), fee.token_id)?;
                        acc_bal += fee.amount;
                        chain.database.update(&[WriteOp::Put(
                            keys::account_balance(&tx_src, fee.token_id),
                            acc_bal.into(),
                        )])?;
                    }

                    let cont_account = chain.get_contract_account(*contract_id)?;

                    chain.database.update(&[WriteOp::Put(
                        keys::compressed_state_at(contract_id, cont_account.height),
                        cont_account.compressed_state.into(),
                    )])?;
                    side_effect = TxSideEffect::StateChange {
                        contract_id: *contract_id,
                        state_change: ZkCompressedStateChange {
                            prev_height: prev_account.height,
                            prev_state: prev_account.compressed_state,
                            state: cont_account.compressed_state,
                        },
                    };
                }
            }

            // Fees go to the Treasury account first
            if tx.src != None {
                let mut treasury_balance =
                    chain.get_balance(Default::default(), tx.fee.token_id)?;
                treasury_balance += tx.fee.amount;
                chain.database.update(&[WriteOp::Put(
                    keys::account_balance(&Default::default(), tx.fee.token_id),
                    treasury_balance.into(),
                )])?;
            }

            Ok(side_effect)
        })?;

        self.database.update(&ops)?;
        Ok(side_effect)
    }

    fn get_changed_states(
        &self,
    ) -> Result<HashMap<ContractId, ZkCompressedStateChange>, BlockchainError> {
        Ok(self
            .database
            .get(keys::contract_updates())?
            .map(|b| b.try_into())
            .ok_or(BlockchainError::Inconsistency)??)
    }

    fn select_transactions(
        &self,
        txs: &[TransactionAndDelta],
        check: bool,
    ) -> Result<Vec<TransactionAndDelta>, BlockchainError> {
        let mut sorted = txs
            .iter()
            .filter(|t| t.tx.fee.token_id == TokenId::Ziesha)
            .cloned()
            .collect::<Vec<_>>();
        // WARN: Sort will be invalid if not all fees are specified in Ziesha
        sorted.sort_unstable_by_key(|tx| {
            let cost = tx.tx.size();
            let is_mpn = if let TransactionData::UpdateContract { contract_id, .. } = &tx.tx.data {
                *contract_id == self.config.mpn_contract_id
            } else {
                false
            };
            (
                is_mpn,
                Into::<u64>::into(tx.tx.fee.amount) / cost as u64,
                -(tx.tx.nonce as i32),
            )
        });
        if !check {
            return Ok(sorted);
        }
        let (_, result) = self.isolated(|chain| {
            let mut result = Vec::new();
            let mut block_sz = 0usize;
            let mut delta_cnt = 0isize;
            for tx in sorted.into_iter().rev() {
                if let Ok((ops, eff)) = chain.isolated(|chain| chain.apply_tx(&tx.tx, false)) {
                    let delta_diff = if let TxSideEffect::StateChange { state_change, .. } = eff {
                        state_change.state.size() as isize - state_change.prev_state.size() as isize
                    } else {
                        0
                    };
                    let block_diff = tx.tx.size();
                    if delta_cnt + delta_diff <= chain.config.max_delta_count as isize
                        && block_sz + block_diff <= chain.config.max_block_size
                        && tx.tx.verify_signature()
                    {
                        delta_cnt += delta_diff;
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

            if let Some(height_limit) = self.config.testnet_height_limit {
                if block.header.number >= height_limit {
                    return Err(BlockchainError::TestnetHeightLimitReached);
                }
            }

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
                // WARN: Sum will be invalid if fees are not in Ziesha
                let fee_sum = Amount(
                    block.body[1..]
                        .iter()
                        .map(|t| -> u64 { t.fee.amount.into() })
                        .sum(),
                );

                let reward_tx = block
                    .body
                    .first()
                    .ok_or(BlockchainError::MinerRewardNotFound)?;

                if reward_tx.src != None
                    || reward_tx.fee != Money::ziesha(0)
                    || reward_tx.sig != Signature::Unsigned
                {
                    return Err(BlockchainError::InvalidMinerReward);
                }
                match &reward_tx.data {
                    TransactionData::RegularSend { entries } => {
                        if entries.len() != 1 {
                            return Err(BlockchainError::InvalidMinerReward);
                        }
                        let reward_entry = &entries[0];

                        if curr_height >= 4675 {
                            if !self.is_validator(
                                block.header.proof_of_work.timestamp,
                                reward_entry.dst.clone(),
                                ValidatorProof {},
                            )? {
                                return Err(BlockchainError::UnelectedValidator);
                            }
                        }

                        if let Some(allowed_miners) = &self.config.limited_miners {
                            if !allowed_miners.contains(&reward_entry.dst) {
                                return Err(BlockchainError::AddressNotAllowedToMine);
                            }
                        }
                        let expected_reward = Money {
                            amount: next_reward + fee_sum,
                            token_id: TokenId::Ziesha,
                        };
                        if reward_entry.amount != expected_reward {
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
            let mut num_mpn_contract_deposits = 0;
            let mut num_mpn_contract_withdraws = 0;

            for tx in txs.iter() {
                // Count MPN updates
                if let TransactionData::UpdateContract {
                    contract_id,
                    updates,
                } = &tx.data
                {
                    if *contract_id == self.config.mpn_contract_id {
                        for update in updates.iter() {
                            match update {
                                ContractUpdate::Deposit { .. } => {
                                    num_mpn_contract_deposits += 1;
                                }
                                ContractUpdate::Withdraw { .. } => {
                                    num_mpn_contract_withdraws += 1;
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
                    if !outdated_contracts.contains(&contract_id) {
                        outdated_contracts.push(contract_id);
                    }
                }
            }

            // TODO: Temporary! Just for testnet
            let func_calls = if curr_height < 3550 {
                self.config.mpn_num_function_calls
            } else {
                10
            };

            if !is_genesis
                && (num_mpn_function_calls < func_calls
                    || num_mpn_contract_deposits < self.config.mpn_num_contract_deposits
                    || num_mpn_contract_withdraws < self.config.mpn_num_contract_withdraws)
            {
                return Err(BlockchainError::InsufficientMpnUpdates);
            }

            if body_size > self.config.max_block_size {
                return Err(BlockchainError::BlockTooBig);
            }

            if state_size_delta > self.config.max_delta_count as isize {
                return Err(BlockchainError::StateDeltaTooBig);
            }

            chain.database.update(&[
                WriteOp::Put(keys::height(), (curr_height + 1).into()),
                WriteOp::Put(
                    keys::power(block.header.number),
                    (block.header.power() + self.get_power()?).into(),
                ),
                WriteOp::Put(
                    keys::header(block.header.number),
                    block.header.clone().into(),
                ),
                WriteOp::Put(keys::block(block.header.number), block.into()),
                WriteOp::Put(
                    keys::merkle(block.header.number),
                    block.merkle_tree().into(),
                ),
                WriteOp::Put(keys::contract_updates(), state_updates.into()),
            ])?;

            let rollback = chain.database.rollback()?;

            chain.database.update(&[
                WriteOp::Put(keys::rollback(block.header.number), rollback.into()),
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

    fn get_power_at(&self, index: u64) -> Result<u128, BlockchainError> {
        if index >= self.get_height()? {
            return Err(BlockchainError::BlockNotFound);
        }
        Ok(self
            .database
            .get(keys::power(index))?
            .ok_or(BlockchainError::Inconsistency)?
            .try_into()?)
    }
}

impl<K: KvStore> Blockchain for KvStoreChain<K> {
    fn db_checksum(&self) -> Result<String, BlockchainError> {
        Ok(hex::encode(self.database.checksum::<Hasher>()?))
    }
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
            let changed_states = chain.get_changed_states()?;

            for (cid, comp) in changed_states {
                if comp.prev_height == 0 {
                    zk::KvStoreStateManager::<CoreZkHasher>::delete_contract(
                        &mut chain.database,
                        cid,
                    )?;
                    outdated.retain(|&x| x != cid);
                    continue;
                }

                if !outdated.contains(&cid) {
                    let (ops, result) = chain.isolated(|fork| {
                        Ok(zk::KvStoreStateManager::<CoreZkHasher>::rollback_contract(
                            &mut fork.database,
                            cid,
                        )?)
                    })?;

                    if result != Some(comp.prev_state) {
                        outdated.push(cid);
                    } else {
                        chain.database.update(&ops)?;
                    }
                } else {
                    let local_compressed_state =
                        zk::KvStoreStateManager::<CoreZkHasher>::root(&chain.database, cid)?;
                    let local_height =
                        zk::KvStoreStateManager::<CoreZkHasher>::height_of(&chain.database, cid)?;
                    if local_compressed_state == comp.prev_state && local_height == comp.prev_height
                    {
                        outdated.retain(|&x| x != cid);
                    }
                }
            }

            chain.database.update(&rollback)?;
            chain.database.update(&[
                WriteOp::Remove(keys::rollback(height - 1)),
                if outdated.is_empty() {
                    WriteOp::Remove(keys::outdated())
                } else {
                    WriteOp::Put(keys::outdated(), outdated.clone().into())
                },
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
            let state_height =
                zk::KvStoreStateManager::<CoreZkHasher>::height_of(&self.database, cid)?;
            ret.insert(cid, state_height);
        }
        Ok(ret)
    }
    fn get_outdated_contracts(&self) -> Result<Vec<ContractId>, BlockchainError> {
        Ok(match self.database.get(keys::outdated())? {
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

    fn get_contract_balance(
        &self,
        contract_id: ContractId,
        token_id: TokenId,
    ) -> Result<Amount, BlockchainError> {
        Ok(
            match self
                .database
                .get(keys::contract_balance(&contract_id, token_id))?
            {
                Some(b) => b.try_into()?,
                None => 0.into(),
            },
        )
    }

    fn get_token(&self, token_id: TokenId) -> Result<Option<Token>, BlockchainError> {
        Ok(match self.database.get(keys::token(&token_id))? {
            Some(b) => Some(b.try_into()?),
            None => None,
        })
    }

    fn get_balance(&self, addr: Address, token_id: TokenId) -> Result<Amount, BlockchainError> {
        Ok(
            match self.database.get(keys::account_balance(&addr, token_id))? {
                Some(b) => b.try_into()?,
                None => 0.into(),
            },
        )
    }

    fn get_account(&self, addr: Address) -> Result<Account, BlockchainError> {
        Ok(match self.database.get(keys::account(&addr))? {
            Some(b) => b.try_into()?,
            None => Account { nonce: 0 },
        })
    }

    fn get_mpn_account(&self, index: u64) -> Result<zk::MpnAccount, BlockchainError> {
        Ok(zk::KvStoreStateManager::<CoreZkHasher>::get_mpn_account(
            &self.database,
            self.config.mpn_contract_id,
            index,
        )?)
    }

    fn get_mpn_accounts(
        &self,
        page: usize,
        page_size: usize,
    ) -> Result<Vec<(u64, zk::MpnAccount)>, BlockchainError> {
        Ok(zk::KvStoreStateManager::<CoreZkHasher>::get_mpn_accounts(
            &self.database,
            self.config.mpn_contract_id,
            page,
            page_size,
        )?)
    }

    fn will_extend(
        &self,
        from: u64,
        headers: &[Header],
        check_pow: bool,
    ) -> Result<bool, BlockchainError> {
        let current_power = self.get_power()?;

        // WARN: Pelmeni Testnet
        let check_pow = from < 4675;

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

        let mut timestamps = (0..std::cmp::min(from, self.config.median_timestamp_count))
            .map(|i| {
                self.get_header(from - 1 - i)
                    .map(|b| b.proof_of_work.timestamp)
            })
            .collect::<Result<Vec<u32>, BlockchainError>>()?;

        let mut diff_timestamps =
            (from.saturating_sub(self.config.difficulty_window + self.config.difficulty_lag)..from)
                .map(|i| self.get_header(i).map(|h| h.proof_of_work.timestamp))
                .collect::<Result<Vec<u32>, BlockchainError>>()?;
        let mut diff_powers =
            (from.saturating_sub(self.config.difficulty_window + self.config.difficulty_lag)..from)
                .map(|i| self.get_power_at(i))
                .collect::<Result<Vec<u128>, BlockchainError>>()?;

        for h in headers.iter() {
            let expected_target = {
                if diff_timestamps.len() <= self.config.difficulty_lag as usize {
                    self.config.minimum_pow_difficulty
                } else {
                    let mut timestamps = diff_timestamps
                        [0..diff_timestamps.len() - self.config.difficulty_lag as usize]
                        .to_vec();
                    if timestamps.len() >= 2 {
                        timestamps.sort_unstable();
                        let final_size =
                            self.config.difficulty_window - 2 * self.config.difficulty_cut;
                        let (begin, end) = if timestamps.len() as u64 > final_size {
                            let begin = (timestamps.len() as u64 - final_size + 1) / 2;
                            let end = begin + final_size - 1;
                            (begin as usize, end as usize)
                        } else {
                            (0, timestamps.len() - 1)
                        };
                        let time_delta = (timestamps[end] - timestamps[begin]).saturating_sub(
                            (end - begin) as u32 * self.config.mpn_proving_time as u32,
                        );
                        if time_delta > 0 {
                            let power_delta = diff_powers[end] - diff_powers[begin];
                            std::cmp::max(
                                Difficulty::from_power(
                                    power_delta * (self.config.block_time as u128)
                                        / (time_delta as u128),
                                ),
                                self.config.minimum_pow_difficulty,
                            )
                        } else {
                            self.config.minimum_pow_difficulty
                        }
                    } else {
                        self.config.minimum_pow_difficulty
                    }
                }
            };
            let pow_key = self.pow_key(h.number)?;

            if h.proof_of_work.timestamp < utils::median(&timestamps) {
                return Err(BlockchainError::InvalidTimestamp);
            }

            if expected_target != h.proof_of_work.target {
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

            timestamps.push(h.proof_of_work.timestamp);
            while timestamps.len() > self.config.median_timestamp_count as usize {
                timestamps.remove(0);
            }

            diff_powers.push(new_power + h.power());
            diff_timestamps.push(h.proof_of_work.timestamp);
            while diff_timestamps.len()
                > (self.config.difficulty_window + self.config.difficulty_lag) as usize
            {
                diff_powers.remove(0);
                diff_timestamps.remove(0);
            }

            last_header = h.clone();
            new_power += h.power();
        }

        Ok(!check_pow || (new_power > current_power))
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
        Ok(match self.database.get(keys::height())? {
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
    fn next_reward(&self) -> Result<Amount, BlockchainError> {
        let supply = self.get_balance(Default::default(), TokenId::Ziesha)?;
        Ok(supply / self.config.reward_ratio)
    }
    fn draft_block(
        &self,
        timestamp: u32,
        mempool: &[TransactionAndDelta],
        wallet: &TxBuilder,
        check: bool,
    ) -> Result<Option<BlockAndPatch>, BlockchainError> {
        let height = self.get_height()?;

        if height >= 4675 {
            if self.validator_status(timestamp, wallet)?.is_none() {
                return Ok(None);
            }
        }
        let outdated_contracts = self.get_outdated_contracts()?;

        if !outdated_contracts.is_empty() {
            return Err(BlockchainError::StatesOutdated);
        }

        let last_header = self.get_header(height - 1)?;
        let treasury_nonce = self.get_account(Default::default())?.nonce;

        let tx_and_deltas = self.select_transactions(mempool, check)?;
        // WARN: Sum will be invalid if tx fees are not in Ziesha!
        let fee_sum = Amount(
            tx_and_deltas
                .iter()
                .map(|t| -> u64 { t.tx.fee.amount.into() })
                .sum(),
        );

        let mut txs = vec![Transaction {
            memo: String::new(),
            src: None,
            data: TransactionData::RegularSend {
                entries: vec![RegularSendEntry {
                    dst: wallet.get_address(),
                    amount: Money {
                        amount: self.next_reward()? + fee_sum,
                        token_id: TokenId::Ziesha,
                    },
                }],
            },
            nonce: treasury_nonce + 1,
            fee: Money::ziesha(0),
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
        if height > 0 {
            self.get_power_at(height - 1)
        } else {
            Ok(0)
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
                        let mut expected_delta_targets = Vec::new();
                        for i in 0..full.rollbacks.len() {
                            expected_delta_targets.push(self.get_compressed_state_at(
                                cid,
                                contract_account.height - 1 - i as u64,
                            )?);
                        }
                        zk::KvStoreStateManager::<CoreZkHasher>::reset_contract(
                            &mut chain.database,
                            cid,
                            contract_account.height,
                            full,
                            &expected_delta_targets,
                        )?;
                    }
                    zk::ZkStatePatch::Delta(delta) => {
                        zk::KvStoreStateManager::<CoreZkHasher>::update_contract(
                            &mut chain.database,
                            cid,
                            delta,
                            contract_account.height,
                        )?;
                    }
                };

                if zk::KvStoreStateManager::<CoreZkHasher>::root(&chain.database, cid)?
                    != contract_account.compressed_state
                {
                    return Err(BlockchainError::FullStateNotValid);
                }
                outdated_contracts.retain(|&x| x != cid);
            }

            chain.database.update(&[if outdated_contracts.is_empty() {
                WriteOp::Remove(keys::outdated())
            } else {
                WriteOp::Put(keys::outdated(), outdated_contracts.clone().into())
            }])?;

            Ok(())
        })?;
        self.database.update(&ops)?;
        Ok(())
    }

    fn cleanup_chain_mempool(&self, mempool: &mut Mempool) -> Result<(), BlockchainError> {
        self.isolated(|chain| {
            let mut txs: Vec<ChainSourcedTx> =
                mempool.chain_sourced().map(|(tx, _)| tx.clone()).collect();
            txs.sort_unstable_by_key(|tx| {
                let not_mpn = if let ChainSourcedTx::TransactionAndDelta(tx) = tx {
                    if let TransactionData::UpdateContract { contract_id, .. } = &tx.tx.data {
                        *contract_id != self.config.mpn_contract_id
                    } else {
                        true
                    }
                } else {
                    true
                };
                (not_mpn, tx.nonce())
            });
            for tx in txs {
                match &tx {
                    ChainSourcedTx::TransactionAndDelta(tx_delta) => {
                        if let Err(e) = chain.apply_tx(&tx_delta.tx, false) {
                            log::info!("Rejecting transaction: {}", e);
                            mempool.reject_chain_sourced(&tx, e);
                        }
                    }
                    ChainSourcedTx::MpnDeposit(mpn_deposit) => {
                        if let Err(e) = chain.apply_mpn_deposit(mpn_deposit) {
                            log::info!("Rejecting mpn-deposit: {}", e);
                            mempool.reject_chain_sourced(&tx, e);
                        }
                    }
                }
            }
            Ok(())
        })?;
        Ok(())
    }

    fn cleanup_mpn_mempool(
        &self,
        mempool: &mut Mempool,
        capacity: usize,
    ) -> Result<(), BlockchainError> {
        self.isolated(|chain| {
            let mut txs = mempool
                .mpn_sourced()
                .map(|(t, s)| (t.clone(), s.clone()))
                .collect::<Vec<_>>();
            txs.sort_unstable_by_key(|(tx, stats)| (!stats.is_local, tx.nonce()));
            for (tx, _) in txs.iter() {
                match tx {
                    MpnSourcedTx::MpnTransaction(mpn_tx) => {
                        if let Err(e) = chain.apply_zero_tx(mpn_tx) {
                            log::info!("Rejecting mpn-transaction: {}", e);
                            mempool.reject_mpn_sourced(tx, e);
                        }
                    }
                    MpnSourcedTx::MpnWithdraw(mpn_withdraw) => {
                        if let Err(e) = chain.apply_mpn_withdraw(mpn_withdraw) {
                            log::info!("Rejecting mpn-withdraw: {}", e);
                            mempool.reject_mpn_sourced(tx, e);
                        }
                    }
                }
            }
            while mempool.mpn_sourced_len() > capacity {
                if let Some(tx) = txs.pop() {
                    mempool.expire_mpn_sourced(&tx.0);
                }
            }
            Ok(())
        })?;
        Ok(())
    }
    fn read_state(
        &self,
        contract_id: ContractId,
        locator: zk::ZkDataLocator,
    ) -> Result<zk::ZkScalar, BlockchainError> {
        Ok(zk::KvStoreStateManager::<CoreZkHasher>::get_data(
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
                let local_height =
                    zk::KvStoreStateManager::<CoreZkHasher>::height_of(&self.database, cid)?;
                if height > local_height {
                    return Err(BlockchainError::StatesUnavailable);
                }
                let away = local_height - height;
                blockchain_patch.patches.insert(
                    cid,
                    if let Some(delta) = zk::KvStoreStateManager::<CoreZkHasher>::delta_of(
                        &self.database,
                        cid,
                        away,
                    )? {
                        zk::ZkStatePatch::Delta(delta)
                    } else {
                        zk::ZkStatePatch::Full(
                            zk::KvStoreStateManager::<CoreZkHasher>::get_full_state(
                                &self.database,
                                cid,
                            )?,
                        )
                    },
                );
            }
        }

        Ok(blockchain_patch)
    }

    fn config(&self) -> &BlockchainConfig {
        &self.config
    }

    fn currency_in_circulation(&self) -> Result<Amount, BlockchainError> {
        let mut amount_sum = Amount(0);
        for (k, v) in self.database.pairs("ACB-".into())? {
            if k.0.ends_with("Ziesha") {
                let bal: Amount = v.try_into().unwrap();
                amount_sum += bal;
            }
        }
        for (k, v) in self.database.pairs("CAB-".into())? {
            if k.0.ends_with("Ziesha") {
                let bal: Amount = v.try_into().unwrap();
                amount_sum += bal;
            }
        }
        Ok(amount_sum)
    }

    fn validator_set(&self) -> Result<Vec<Address>, BlockchainError> {
        Ok(if self.get_height()? >= 5000 {
            vec![
                "0xac798dca2e3275b06948c6839b9813697fc2fb60174c79e366f860d844b17202", // Apricot Pool
                "0x115845ae1e3565a956b329285ae2f28d53e4b67865ad92f0131c46bea76cf858", // CHN Pool
                "0x078800541138668423cbad38275209481583b8d9fd12bd03d5f859805b054db6", // Starnodes Pool
                "0x2d5bd425ecdb9c64f098216271ea19bfecc79a8de76a8b402b5260425a9114bc", // Nodn Pool
                "0x5bbc3a8fd8dc14fe7a93211e3b2f4cca8c198c5c4a64fa1e097251b087759a14", // LazyPenguin Pool
                "0x9ab51573c41c03d7ab863569df608daecbfe0b2b0da0efe282fd8f68e464ce28", // Making.Cash Pool
                "0xeaac78d4d14d59e727a903f8187fc25020022869e8c61ba9443a86fddc819ae6", // Super One Pool
                "0x8d2fdd62ec6c067789aa694cee59b81f34aa354fcb461acada43e26770f41d36", // Systemd Pool
                "0xd1350235d42dfd428b592b7badfe1538aec9a3b11393a0f639ff400045255eda",
                "0xa601420e26c8461ababf6314eed8589e0e6dec04015006e528fdff704e817e12",
                "0x923f518f4635194f562dbaaed6f6893dbec2b8b1cfddd3cf8c843adff498daf7",
                "0xa8e819854ec66bb13194f7d202574484d764836a00ce88c79da259898c89debe",
                "0x159defb2be4ee8bc88592328bd4594e366bdccef75949276d7f94170a97da8b5",
                "0x311a53d3b84f86ed6367c976be35a6587115786c2ffe33a8c1779b71dfe87f65",
                "0x6477379d43c0eca965e6567d573eeaf0797a5799cc598bab7e66ac289edb153b",
                "0x7af5d18274de5d80a9618401cd517f9124a640ca0ff8f4dab38508f24a1438ca",
                "0xf70f00d0e166c67ac9b4825d65545082f396c3aa3de3a0507f56654e6ab9eba3",
                "0x632e30fc6bc3017a4a05cd985c04e47be724cb98f7cfc0f31687a4c166fd855a",
                "0x8f06b23a4598bb2e263ceb5013287138d891752cc725a653827d54251bc6a3f8",
                "0x5c668535143381256a0beaafb3444c8e34950324f50c589be7880771b6d55dfc",
                "0xc23d5fa2ec9da01fe80a7a91e31faca358c462e1df6467c9988005755ac5479a",
                "0x11176f82e7ac98baa95180e40d568bdee21bae1da5add2eae323db69e61d10f6",
                "0x8ff3c29c476cb0fda1647b05f20b56d22b85b4eb10411e91275668c8ceac8a42",
                "0xb33fc644b44a04299605ad6f8b16e5e517bf09f2be4bd517dac3660650049df7",
                "0x478c5c2b67292eff1d1b2cb1f1c9afd16b3789050a1dd6bcd5a46f6b283db68f",
                "0x5c76767e69f15bacc30c58251690fc075963b04bc08d314fe015cbc0e861f3d7",
                "0x3458d3b32aac6b23ee5c5be1f523243b0101356f329f68075caf8683cf6bff4d",
                "0x1e1cfa41c4546684d5bda8c78161cb654d4e45fdd68a207b976ca302bf6b05b6",
                "0x31f2e519d335be5c941df424bca8fe5943bddd1c2c96e21f52cd5cf9f6058041",
                "0x7f7fd6bb6497f41cdd262bcd61035fd67db71f94e6bcba92fa7b83edebef0e0f",
                "0x8d61b4ebdf40c356b51f0413ba33eb3a5cf10978283b79a645f25d12016e8074",
                "0x6ce694bc947fdcd373848137aee27670451cd21d4804659a2dc939cde3e5e3cf",
                "0xc4303f8e539efa15dca0436fc60f15836a76a3735bd63bbad5034ff248dd8b47",
                "0xf2c401817b84497e04d344a3285cea8d114445c93aa7beb6225895fedf4eedd3",
                "0xfdaf9489a5815679b6aaca7288bca04d01e01c0fe955b2b97985ee5e6b035d0d",
                "0xe495f73172a08324a2c98be4d56c9f93f5654d5fb22d2b0caab5d6065733b2de",
                "0x1183ed0046a59b26df9044b3ee3083435585f9198cb1b68ad5a91e4ea56bc4da",
                "0x737b3e5fd5eab898e7efec141a9605b0c6470a182e35ee55943d344565a872b6",
                "0x4fdba9a5b2e407a4f310c669366432b306ee1b949f75156fdf52451efe76cb26",
                "0xae00fbe3cc48152e268d2747a502ac97282fccc2168dc857bbe64f297fb43078",
                "0xd2ac08bf81505c8ce18e4343f361cd533848c5b567de0e11c4d17d8d067a402b",
                "0xe98eb0e4965453f9da5c2b35a7fe88f994a260a27394abc46905ad8df8d85ee7",
            ]
        } else {
            vec![
                "0xac798dca2e3275b06948c6839b9813697fc2fb60174c79e366f860d844b17202", // Apricot Pool
                "0x115845ae1e3565a956b329285ae2f28d53e4b67865ad92f0131c46bea76cf858", // CHN Pool
                "0x078800541138668423cbad38275209481583b8d9fd12bd03d5f859805b054db6", // Starnodes Pool
                "0x2d5bd425ecdb9c64f098216271ea19bfecc79a8de76a8b402b5260425a9114bc", // Nodn Pool
                "0x5bbc3a8fd8dc14fe7a93211e3b2f4cca8c198c5c4a64fa1e097251b087759a14", // LazyPenguin Pool
                "0x9ab51573c41c03d7ab863569df608daecbfe0b2b0da0efe282fd8f68e464ce28", // Making.Cash Pool
                "0xeaac78d4d14d59e727a903f8187fc25020022869e8c61ba9443a86fddc819ae6", // Super One Pool
                "0x8d2fdd62ec6c067789aa694cee59b81f34aa354fcb461acada43e26770f41d36", // Systemd Pool
            ]
        }
        .iter()
        .map(|s| s.parse().unwrap())
        .collect())
    }
    fn is_validator(
        &self,
        timestamp: u32,
        addr: Address,
        _proof: ValidatorProof,
    ) -> Result<bool, BlockchainError> {
        let slot_number = (timestamp / 180) as usize;
        let i = slot_number % self.validator_set()?.len();
        let winner = self.validator_set()?[i].clone();
        Ok(addr == winner)
    }
    fn validator_status(
        &self,
        timestamp: u32,
        wallet: &TxBuilder,
    ) -> Result<Option<ValidatorProof>, BlockchainError> {
        let slot_number = (timestamp / 180) as usize;
        let i = slot_number % self.validator_set()?.len();
        let winner = self.validator_set()?[i].clone();
        Ok(if wallet.get_address() == winner {
            Some(ValidatorProof {})
        } else {
            None
        })
    }
}

#[cfg(test)]
mod test;
