mod error;
pub use error::*;

use crate::consensus::pow::Difficulty;
use crate::core::{
    hash::Hash, Account, Address, Amount, Block, ChainSourcedTx, ContractAccount, ContractDeposit,
    ContractId, ContractUpdate, ContractWithdraw, Delegate, Hasher, Header, Money, MpnDeposit,
    MpnSourcedTx, MpnWithdraw, ProofOfWork, RegularSendEntry, Signature, Staker, Token, TokenId,
    TokenUpdate, Transaction, TransactionAndDelta, TransactionData, ZkHasher as CoreZkHasher,
};
use crate::crypto::ZkSignatureScheme;
use crate::db::{keys, KvStore, RamMirrorKvStore, WriteOp};
use crate::utils;
use crate::wallet::TxBuilder;
use crate::zk;
use crate::zk::ZkHasher;

use rayon::prelude::*;

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Default)]
pub struct Mempool {
    chain_sourced: HashMap<ChainSourcedTx, TransactionStats>,
    rejected_chain_sourced: HashMap<ChainSourcedTx, TransactionStats>,
    mpn_sourced: HashMap<MpnSourcedTx, TransactionStats>,
    rejected_mpn_sourced: HashMap<MpnSourcedTx, TransactionStats>,
}

impl Mempool {
    pub fn refresh(
        &mut self,
        local_ts: u32,
        max_time_alive: Option<u32>,
        max_time_remember: Option<u32>,
    ) {
        if let Some(max_time_alive) = max_time_alive {
            for (tx, stats) in self.chain_sourced.clone().into_iter() {
                if local_ts - stats.first_seen > max_time_alive {
                    self.expire_chain_sourced(&tx);
                }
            }
            for (tx, stats) in self.mpn_sourced.clone().into_iter() {
                if local_ts - stats.first_seen > max_time_alive {
                    self.expire_mpn_sourced(&tx);
                }
            }
        }
        if let Some(max_time_remember) = max_time_remember {
            for (tx, stats) in self.rejected_chain_sourced.clone().into_iter() {
                if local_ts - stats.first_seen > max_time_remember {
                    self.rejected_chain_sourced.remove(&tx);
                }
            }
            for (tx, stats) in self.rejected_mpn_sourced.clone().into_iter() {
                if local_ts - stats.first_seen > max_time_remember {
                    self.rejected_mpn_sourced.remove(&tx);
                }
            }
        }
    }
    pub fn add_chain_sourced(&mut self, tx: ChainSourcedTx, stats: TransactionStats) {
        if !self.rejected_chain_sourced.contains_key(&tx) {
            self.chain_sourced.insert(tx, stats);
        }
    }
    pub fn add_mpn_sourced(&mut self, tx: MpnSourcedTx, stats: TransactionStats) {
        if !self.rejected_mpn_sourced.contains_key(&tx) {
            self.mpn_sourced.insert(tx, stats);
        }
    }
    pub fn reject_chain_sourced(&mut self, tx: &ChainSourcedTx) {
        if let Some(stats) = self.chain_sourced.remove(tx) {
            self.rejected_chain_sourced.insert(tx.clone(), stats);
        }
    }
    pub fn reject_mpn_sourced(&mut self, tx: &MpnSourcedTx) {
        if let Some(stats) = self.mpn_sourced.remove(tx) {
            self.rejected_mpn_sourced.insert(tx.clone(), stats);
        }
    }
    pub fn expire_chain_sourced(&mut self, tx: &ChainSourcedTx) {
        self.reject_chain_sourced(tx);
    }
    pub fn expire_mpn_sourced(&mut self, tx: &MpnSourcedTx) {
        self.reject_mpn_sourced(tx);
    }
    pub fn chain_sourced(&self) -> &HashMap<ChainSourcedTx, TransactionStats> {
        &self.chain_sourced
    }
    pub fn mpn_sourced(&self) -> &HashMap<MpnSourcedTx, TransactionStats> {
        &self.mpn_sourced
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

pub trait Blockchain {
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
    fn get_delegate_of(&self, from: Address, to: Address) -> Result<Delegate, BlockchainError>;
    fn get_staker(&self, addr: Address) -> Result<Option<Staker>, BlockchainError>;
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
            let calldata = CoreZkHasher::hash(&[
                withdraw.zk_address.decompress().0,
                withdraw.zk_address.decompress().1,
                zk::ZkScalar::from(withdraw.zk_nonce),
                withdraw.zk_sig.r.0,
                withdraw.zk_sig.r.1,
                withdraw.zk_sig.s,
            ]);
            let msg = CoreZkHasher::hash(&[
                withdraw.payment.fingerprint(),
                zk::ZkScalar::from(withdraw.zk_nonce as u64),
            ]);
            if withdraw.zk_address_index(self.config.mpn_log4_account_capacity) > 0x3FFFFFFF
                || withdraw.zk_token_index >= 64
                || withdraw.zk_nonce != src.nonce
                || withdraw.payment.calldata != calldata
                || !crate::core::ZkSigner::verify(&withdraw.zk_address, msg, &withdraw.zk_sig)
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
            let src = chain.get_mpn_account(tx.src_index(self.config.mpn_log4_account_capacity))?;
            let dst = chain.get_mpn_account(tx.dst_index(self.config.mpn_log4_account_capacity))?;
            if tx.src_index(self.config.mpn_log4_account_capacity) > 0x3FFFFFFF
                || tx.dst_index(self.config.mpn_log4_account_capacity) > 0x3FFFFFFF
                || tx.src_index(self.config.mpn_log4_account_capacity)
                    == tx.dst_index(self.config.mpn_log4_account_capacity)
                || !tx.src_pub_key.is_on_curve()
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
                tx.src_index(self.config.mpn_log4_account_capacity),
                new_src_acc,
                &mut size_diff,
            )?;
            zk::KvStoreStateManager::<CoreZkHasher>::set_mpn_account(
                &mut chain.database,
                chain.config.mpn_contract_id,
                tx.dst_index(self.config.mpn_log4_account_capacity),
                new_dst_acc,
                &mut size_diff,
            )?;
            Ok(())
        })?;
        self.database.update(&ops)?;
        Ok(())
    }

    fn get_stakers(&self) -> Result<Vec<Staker>, BlockchainError> {
        let mut stakers = self
            .database
            .pairs(keys::staker_prefix().into())?
            .into_iter()
            .map(|(k, v)| {
                || -> Result<(Address, Staker), BlockchainError> {
                    let pk: Address = k.0[4..]
                        .parse()
                        .map_err(|_| BlockchainError::Inconsistency)?;
                    let v: Staker = v.try_into()?;
                    Ok((pk, v))
                }()
            })
            .collect::<Result<Vec<_>, _>>()?;
        stakers.sort_unstable_by_key(|(_, v)| v.stake);
        Ok(stakers.into_iter().map(|(_, v)| v).collect())
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
                TransactionData::RegisterStaker { vrf_pub_key } => {
                    if chain.get_staker(tx_src.clone())?.is_none() {
                        chain.database.update(&[WriteOp::Put(
                            keys::staker(&tx_src),
                            Staker {
                                stake: Amount(0),
                                vrf_pub_key: vrf_pub_key.clone(),
                            }
                            .into(),
                        )])?;
                    } else {
                        return Err(BlockchainError::StakerAlreadyRegistered);
                    }
                }
                TransactionData::Delegate {
                    amount,
                    to,
                    reverse,
                } => {
                    if let Some(mut acc_del) = chain.get_staker(to.clone())? {
                        let mut src_bal = chain.get_balance(tx_src.clone(), TokenId::Ziesha)?;
                        let mut del = chain.get_delegate_of(tx_src.clone(), to.clone())?;
                        if !reverse {
                            if src_bal < *amount {
                                return Err(BlockchainError::BalanceInsufficient);
                            }
                            src_bal -= *amount;
                            del.amount += *amount;
                            acc_del.stake += *amount;
                        } else {
                            if del.amount < *amount {
                                return Err(BlockchainError::BalanceInsufficient);
                            }
                            del.amount -= *amount;
                            acc_del.stake -= *amount;
                            src_bal += *amount;
                        }
                        chain.database.update(&[WriteOp::Put(
                            keys::account_balance(&tx_src, TokenId::Ziesha),
                            src_bal.into(),
                        )])?;
                        chain
                            .database
                            .update(&[WriteOp::Put(keys::staker(&to), acc_del.into())])?;
                        chain
                            .database
                            .update(&[WriteOp::Put(keys::delegate(&tx_src, to), del.into())])?;
                    } else {
                        return Err(BlockchainError::StakerNotFound);
                    }
                }
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

    fn get_staker(&self, addr: Address) -> Result<Option<Staker>, BlockchainError> {
        Ok(match self.database.get(keys::staker(&addr))? {
            Some(b) => Some(b.try_into()?),
            None => None,
        })
    }

    fn get_delegate_of(&self, from: Address, to: Address) -> Result<Delegate, BlockchainError> {
        Ok(match self.database.get(keys::delegate(&from, &to))? {
            Some(b) => b.try_into()?,
            None => Delegate { amount: Amount(0) },
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
            let mut txs: Vec<ChainSourcedTx> = mempool
                .chain_sourced()
                .clone()
                .into_iter()
                .filter_map(|(tx, stats)| {
                    if stats.validity != TransactionValidity::Invalid {
                        Some(tx)
                    } else {
                        None
                    }
                })
                .collect();
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
                            mempool.reject_chain_sourced(&tx);
                        }
                    }
                    ChainSourcedTx::MpnDeposit(mpn_deposit) => {
                        if let Err(e) = chain.apply_mpn_deposit(mpn_deposit) {
                            log::info!("Rejecting mpn-deposit: {}", e);
                            mempool.reject_chain_sourced(&tx);
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
                .mpn_sourced
                .clone()
                .into_iter()
                .filter(|(_, stats)| stats.validity != TransactionValidity::Invalid)
                .collect::<Vec<_>>();
            txs.sort_unstable_by_key(|(tx, stats)| (!stats.is_local, tx.nonce()));
            for (tx, _) in txs.iter() {
                match tx {
                    MpnSourcedTx::MpnTransaction(mpn_tx) => {
                        if let Err(e) = chain.apply_zero_tx(mpn_tx) {
                            log::info!("Rejecting mpn-transaction: {}", e);
                            mempool.reject_mpn_sourced(tx);
                        }
                    }
                    MpnSourcedTx::MpnWithdraw(mpn_withdraw) => {
                        if let Err(e) = chain.apply_mpn_withdraw(mpn_withdraw) {
                            log::info!("Rejecting mpn-withdraw: {}", e);
                            mempool.reject_mpn_sourced(tx);
                        }
                    }
                }
            }
            while mempool.mpn_sourced.len() > capacity {
                if let Some(tx) = txs.pop() {
                    mempool.expire_mpn_sourced(&tx.0);
                }
            }
            for (t, _) in mempool
                .mpn_sourced
                .clone()
                .into_par_iter()
                .filter(|(t, _)| match t {
                    MpnSourcedTx::MpnTransaction(t) => !t.verify(),
                    _ => false,
                })
                .collect::<Vec<_>>()
            {
                mempool.mpn_sourced.remove(&t);
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
}

#[cfg(test)]
mod test;
