mod error;
pub use error::*;

use crate::core::{
    hash::Hash, Account, Address, Amount, Block, ChainSourcedTx, ContractAccount, ContractDeposit,
    ContractId, ContractUpdate, ContractWithdraw, Delegate, Hasher, Header, Money, MpnAddress,
    MpnSourcedTx, ProofOfStake, RegularSendEntry, Signature, Staker, Token, TokenId, TokenUpdate,
    Transaction, TransactionAndDelta, TransactionData, ValidatorProof, Vrf,
    ZkHasher as CoreZkHasher,
};
use crate::crypto::VerifiableRandomFunction;
use crate::db::{keys, KvStore, RamMirrorKvStore, WriteOp};
use crate::mpn::MpnConfig;
use crate::wallet::TxBuilder;
use crate::zk;

use rayon::prelude::*;

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone)]
pub struct MpnAccountMempool {
    account: zk::MpnAccount,
    txs: VecDeque<(MpnSourcedTx, TransactionStats)>,
}

impl MpnAccountMempool {
    fn new(account: zk::MpnAccount) -> Self {
        Self {
            account,
            txs: Default::default(),
        }
    }
    fn len(&self) -> usize {
        self.txs.len()
    }
    fn first_nonce(&self) -> Option<u64> {
        self.txs.front().map(|(tx, _)| tx.nonce())
    }
    fn last_nonce(&self) -> Option<u64> {
        self.txs.back().map(|(tx, _)| tx.nonce())
    }
    fn applicable(&self, tx: &MpnSourcedTx) -> bool {
        if let Some(last_nonce) = self.last_nonce() {
            tx.nonce() == last_nonce + 1
        } else {
            self.account.nonce == tx.nonce()
        }
    }
    fn insert(&mut self, tx: MpnSourcedTx, stats: TransactionStats) {
        if self.applicable(&tx) {
            self.txs.push_back((tx, stats));
        }
    }
    fn update_account(&mut self, account: zk::MpnAccount) {
        while let Some(first_nonce) = self.first_nonce() {
            if first_nonce < account.nonce {
                self.txs.pop_front();
            } else {
                break;
            }
        }
        if self.first_nonce() != Some(account.nonce) {
            self.txs.clear();
        }
        self.account = account;
    }
}

#[derive(Debug, Clone)]
pub struct AccountMempool {
    account: Account,
    txs: VecDeque<(ChainSourcedTx, TransactionStats)>,
}

impl AccountMempool {
    fn new(account: Account) -> Self {
        Self {
            account,
            txs: Default::default(),
        }
    }
    fn len(&self) -> usize {
        self.txs.len()
    }
    fn first_nonce(&self) -> Option<u32> {
        self.txs.front().map(|(tx, _)| tx.nonce())
    }
    fn last_nonce(&self) -> Option<u32> {
        self.txs.back().map(|(tx, _)| tx.nonce())
    }
    fn applicable(&self, tx: &ChainSourcedTx) -> bool {
        if let Some(last_nonce) = self.last_nonce() {
            tx.nonce() == last_nonce + 1
        } else {
            self.account.nonce + 1 == tx.nonce()
        }
    }
    fn reset(&mut self, nonce: u32) {
        if nonce == 0 {
            self.txs.clear();
            return;
        }
        while let Some(last_nonce) = self.last_nonce() {
            if last_nonce > nonce - 1 {
                self.txs.pop_back();
            } else {
                break;
            }
        }
        if self.last_nonce() != Some(nonce - 1) {
            self.txs.clear();
        }
    }
    fn insert(&mut self, tx: ChainSourcedTx, stats: TransactionStats) {
        if self.applicable(&tx) {
            self.txs.push_back((tx, stats));
        }
    }
    fn update_account(&mut self, account: Account) {
        while let Some(first_nonce) = self.first_nonce() {
            if first_nonce <= account.nonce {
                self.txs.pop_front();
            } else {
                break;
            }
        }
        if self.first_nonce() != Some(account.nonce + 1) {
            self.txs.clear();
        }
        self.account = account;
    }
}

#[derive(Clone, Debug)]
pub struct Mempool {
    mpn_log4_account_capacity: u8,
    chain_sourced: HashMap<Address, AccountMempool>,
    rejected_chain_sourced: HashMap<ChainSourcedTx, TransactionStats>,
    mpn_sourced: HashMap<MpnAddress, MpnAccountMempool>,
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
    #[allow(dead_code)]
    pub fn refresh<K: KvStore, B: Blockchain<K>>(
        &mut self,
        _blockchain: &B,
        _local_ts: u32,
        _max_time_alive: Option<u32>,
        _max_time_remember: Option<u32>,
    ) -> Result<(), BlockchainError> {
        Ok(())
    }
    pub fn chain_address_limit(&self, _addr: Address) -> usize {
        100
    }
    pub fn mpn_sourced_len(&self) -> usize {
        self.mpn_sourced.values().map(|c| c.len()).sum()
    }
    pub fn chain_sourced_len(&self) -> usize {
        self.chain_sourced.values().map(|c| c.len()).sum()
    }
    pub fn add_chain_sourced<K: KvStore, B: Blockchain<K>>(
        &mut self,
        blockchain: &B,
        tx: ChainSourcedTx,
        is_local: bool,
        now: u32,
    ) -> Result<(), BlockchainError> {
        if is_local {
            self.rejected_chain_sourced.remove(&tx);
        }
        if !self.rejected_chain_sourced.contains_key(&tx) {
            let chain_acc = blockchain.get_account(tx.sender())?;
            if self
                .chain_sourced
                .get_mut(&tx.sender())
                .map(|all| {
                    all.update_account(chain_acc.clone());
                    if is_local && !all.applicable(&tx) {
                        all.reset(tx.nonce());
                    }
                    !all.applicable(&tx)
                })
                .unwrap_or_default()
            {
                return Ok(());
            }

            // Do not accept old txs in the mempool
            if tx.nonce() <= chain_acc.nonce {
                return Ok(());
            }

            let ziesha_balance = blockchain.get_balance(tx.sender(), TokenId::Ziesha)?;

            // Allow 1tx in mempool per Ziesha
            // Min: 1 Max: 1000
            let limit = std::cmp::max(
                std::cmp::min(Into::<u64>::into(ziesha_balance) / 1000000000, 1000),
                1,
            ) as usize;

            let all = self
                .chain_sourced
                .entry(tx.sender().clone())
                .or_insert(AccountMempool::new(chain_acc));
            if tx.verify_signature() {
                if is_local || all.len() < limit {
                    all.insert(tx.clone(), TransactionStats::new(is_local, now));
                }
            }
        }
        Ok(())
    }
    pub fn add_mpn_sourced<K: KvStore, B: Blockchain<K>>(
        &mut self,
        blockchain: &B,
        tx: MpnSourcedTx,
        is_local: bool,
        now: u32,
    ) -> Result<(), BlockchainError> {
        if is_local {
            self.rejected_mpn_sourced.remove(&tx);
        }
        if !self.rejected_mpn_sourced.contains_key(&tx) {
            let mpn_acc = blockchain
                .get_mpn_account(tx.sender().account_index(self.mpn_log4_account_capacity))?;
            if self
                .mpn_sourced
                .get_mut(&tx.sender())
                .map(|all| {
                    all.update_account(mpn_acc.clone());
                    !all.applicable(&tx)
                })
                .unwrap_or_default()
            {
                return Ok(());
            }

            // Do not accept txs from non-existing accounts
            if tx.sender().pub_key.0.decompress() != mpn_acc.address {
                return Ok(());
            }

            // Do not accept old txs in the mempool
            if tx.nonce() < mpn_acc.nonce {
                return Ok(());
            }

            // Do not accept txs coming from accounts that their 0th slot has no Ziesha
            if let Some(money) = mpn_acc.tokens.get(&0) {
                if money.token_id != TokenId::Ziesha {
                    return Ok(());
                }

                // Allow 1tx in mempool per Ziesha
                // Min: 1 Max: 1000
                let limit = std::cmp::max(
                    std::cmp::min(Into::<u64>::into(money.amount) / 1000000000, 1000),
                    1,
                ) as usize;

                let all = self
                    .mpn_sourced
                    .entry(tx.sender().clone())
                    .or_insert(MpnAccountMempool::new(mpn_acc));
                if tx.verify_signature() {
                    if is_local || all.len() < limit {
                        all.insert(tx.clone(), TransactionStats::new(is_local, now));
                    }
                }
            }
        }
        Ok(())
    }
    pub fn chain_sourced(&self) -> impl Iterator<Item = &(ChainSourcedTx, TransactionStats)> {
        self.chain_sourced
            .iter()
            .map(|(_, c)| c.txs.iter())
            .flatten()
    }
    pub fn mpn_sourced(&self) -> impl Iterator<Item = &(MpnSourcedTx, TransactionStats)> {
        self.mpn_sourced.iter().map(|(_, c)| c.txs.iter()).flatten()
    }
}

#[derive(Clone)]
pub struct BlockchainConfig {
    pub limited_miners: Option<HashSet<Address>>,
    pub genesis: BlockAndPatch,
    pub reward_ratio: u64,
    pub max_block_size: usize,
    pub max_delta_count: usize,
    pub ziesha_token_id: TokenId,
    pub mpn_config: MpnConfig,
    pub testnet_height_limit: Option<u64>,
    pub max_memo_length: usize,
    pub slot_duration: u32,
    pub slot_per_epoch: u32,
    pub chain_start_timestamp: u32,
    pub check_validator: bool,
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

pub trait Blockchain<K: KvStore> {
    fn database(&self) -> &K;
    fn epoch_slot(&self, timestamp: u32) -> (u32, u32);
    fn get_stake(&self, addr: Address) -> Result<Amount, BlockchainError>;
    fn get_stakers(&self) -> Result<Vec<(Address, Amount)>, BlockchainError>;
    fn get_delegatees(
        &self,
        delegator: Address,
        top: usize,
    ) -> Result<Vec<(Address, Amount)>, BlockchainError>;
    fn get_delegators(&self, delegatee: Address)
        -> Result<Vec<(Address, Amount)>, BlockchainError>;
    fn cleanup_chain_mempool(
        &self,
        txs: &[ChainSourcedTx],
    ) -> Result<Vec<ChainSourcedTx>, BlockchainError>;
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
    ) -> Result<ValidatorProof, BlockchainError>;

    fn currency_in_circulation(&self) -> Result<Amount, BlockchainError>;

    fn config(&self) -> &BlockchainConfig;

    fn db_checksum(&self) -> Result<String, BlockchainError>;

    fn get_token(&self, token_id: TokenId) -> Result<Option<Token>, BlockchainError>;

    fn get_balance(&self, addr: Address, token_id: TokenId) -> Result<Amount, BlockchainError>;
    fn get_contract_balance(
        &self,
        contract_id: ContractId,
        token_id: TokenId,
    ) -> Result<Amount, BlockchainError>;
    fn get_delegate(
        &self,
        delegator: Address,
        delegatee: Address,
    ) -> Result<Delegate, BlockchainError>;
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
    fn will_extend(&self, from: u64, headers: &[Header]) -> Result<bool, BlockchainError>;
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
            chain.apply_block(&config.genesis.block)?;
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
                TransactionData::UpdateStaker { vrf_pub_key } => {
                    chain.database.update(&[WriteOp::Put(
                        keys::staker(&tx_src),
                        Staker {
                            vrf_pub_key: vrf_pub_key.clone(),
                        }
                        .into(),
                    )])?;
                }
                TransactionData::Delegate {
                    amount,
                    to,
                    reverse,
                } => {
                    let mut src_bal = chain.get_balance(tx_src.clone(), TokenId::Ziesha)?;
                    if !reverse {
                        if src_bal < *amount {
                            return Err(BlockchainError::BalanceInsufficient);
                        }
                        src_bal -= *amount;
                    } else {
                        src_bal += *amount;
                    }
                    chain.database.update(&[WriteOp::Put(
                        keys::account_balance(&tx_src, TokenId::Ziesha),
                        src_bal.into(),
                    )])?;

                    let mut delegate = self.get_delegate(tx_src.clone(), to.clone())?;
                    let old_delegate = delegate.amount;
                    if !reverse {
                        delegate.amount += *amount;
                    } else {
                        if delegate.amount < *amount {
                            return Err(BlockchainError::BalanceInsufficient);
                        }
                        delegate.amount -= *amount;
                    }
                    chain.database.update(&[WriteOp::Put(
                        keys::delegate(&tx_src, to),
                        delegate.clone().into(),
                    )])?;

                    let old_stake = chain.get_stake(to.clone())?;
                    let new_stake = old_stake + *amount;
                    chain.database.update(&[
                        WriteOp::Remove(keys::delegatee_rank(&tx_src, old_delegate, &to)),
                        WriteOp::Put(
                            keys::delegatee_rank(&tx_src, delegate.amount, &to),
                            ().into(),
                        ),
                        WriteOp::Remove(keys::delegator_rank(&to, old_delegate, &tx_src)),
                        WriteOp::Put(
                            keys::delegator_rank(&to, delegate.amount, &tx_src),
                            ().into(),
                        ),
                        WriteOp::Remove(keys::staker_rank(old_stake, &to)),
                        WriteOp::Put(keys::staker_rank(new_stake, &to), ().into()),
                        WriteOp::Put(keys::stake(&to), new_stake.into()),
                    ])?;
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
                *contract_id == self.config.mpn_config.mpn_contract_id
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
                match chain.isolated(|chain| chain.apply_tx(&tx.tx, false)) {
                    Ok((ops, eff)) => {
                        let delta_diff = if let TxSideEffect::StateChange { state_change, .. } = eff
                        {
                            state_change.state.size() as isize
                                - state_change.prev_state.size() as isize
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
                    Err(e) => {
                        let is_mpn = if let TransactionData::UpdateContract {
                            contract_id, ..
                        } = &tx.tx.data
                        {
                            *contract_id == self.config.mpn_config.mpn_contract_id
                        } else {
                            false
                        };
                        if is_mpn {
                            log::error!("MPN transaction rejected: {}", e);
                        }
                    }
                }
            }
            Ok(result)
        })?;
        Ok(result)
    }

    fn apply_block(&mut self, block: &Block) -> Result<(), BlockchainError> {
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

                chain.will_extend(curr_height, &[block.header.clone()])?;
            }

            if !is_genesis {
                if chain.config.check_validator
                    && !chain.is_validator(
                        block.header.proof_of_stake.timestamp,
                        block.header.proof_of_stake.validator.clone(),
                        block.header.proof_of_stake.proof.clone(),
                    )?
                {
                    return Err(BlockchainError::UnelectedValidator);
                }

                // WARN: Sum will be invalid if fees are not in Ziesha
                let fee_sum = Amount(
                    block
                        .body
                        .iter()
                        .map(|t| -> u64 { t.fee.amount.into() })
                        .sum(),
                );
                let treasury_nonce = chain.get_account(Default::default())?.nonce;

                // Reward tx allowed to get money from Treasury
                chain.apply_tx(
                    &Transaction {
                        memo: String::new(),
                        src: None,
                        data: TransactionData::RegularSend {
                            entries: vec![RegularSendEntry {
                                dst: block.header.proof_of_stake.validator.clone(),
                                amount: Money {
                                    amount: next_reward + fee_sum,
                                    token_id: TokenId::Ziesha,
                                },
                            }],
                        },
                        nonce: treasury_nonce + 1,
                        fee: Money::ziesha(0),
                        sig: Signature::Unsigned,
                    },
                    true,
                )?;
            }

            let mut body_size = 0usize;
            let mut state_size_delta = 0isize;
            let mut state_updates: HashMap<ContractId, ZkCompressedStateChange> = HashMap::new();
            let mut outdated_contracts = self.get_outdated_contracts()?;

            if !is_genesis && !block.body.par_iter().all(|tx| tx.verify_signature()) {
                return Err(BlockchainError::SignatureError);
            }

            let mut num_mpn_function_calls = 0;
            let mut num_mpn_contract_deposits = 0;
            let mut num_mpn_contract_withdraws = 0;

            for tx in block.body.iter() {
                // Count MPN updates
                if let TransactionData::UpdateContract {
                    contract_id,
                    updates,
                } = &tx.data
                {
                    if contract_id.clone() == self.config.mpn_config.mpn_contract_id {
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
                    if !state_updates.contains_key(&contract_id) {
                        state_updates.insert(contract_id, state_change.clone());
                    } else {
                        return Err(BlockchainError::SingleUpdateAllowedPerContract);
                    }
                    if !outdated_contracts.contains(&contract_id) {
                        outdated_contracts.push(contract_id);
                    }
                }
            }

            if !is_genesis
                && (num_mpn_function_calls < self.config.mpn_config.mpn_num_update_batches
                    || num_mpn_contract_deposits < self.config.mpn_config.mpn_num_deposit_batches
                    || num_mpn_contract_withdraws < self.config.mpn_config.mpn_num_withdraw_batches)
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
}

impl<K: KvStore> Blockchain<K> for KvStoreChain<K> {
    fn db_checksum(&self) -> Result<String, BlockchainError> {
        Ok(hex::encode(
            self.database.pairs("".into())?.checksum::<Hasher>()?,
        ))
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

    fn get_delegate(
        &self,
        delegator: Address,
        delegatee: Address,
    ) -> Result<Delegate, BlockchainError> {
        Ok(
            match self.database.get(keys::delegate(&delegator, &delegatee))? {
                Some(b) => b.try_into()?,
                None => Delegate { amount: Amount(0) },
            },
        )
    }

    fn get_mpn_account(&self, index: u64) -> Result<zk::MpnAccount, BlockchainError> {
        Ok(zk::KvStoreStateManager::<CoreZkHasher>::get_mpn_account(
            &self.database,
            self.config.mpn_config.mpn_contract_id,
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
            self.config.mpn_config.mpn_contract_id,
            page,
            page_size,
        )?)
    }

    fn will_extend(&self, from: u64, headers: &[Header]) -> Result<bool, BlockchainError> {
        if from + headers.len() as u64 <= self.get_height()? {
            return Ok(false);
        }

        if from == 0 {
            return Err(BlockchainError::ExtendFromGenesis);
        } else if from > self.get_height()? {
            return Err(BlockchainError::ExtendFromFuture);
        }

        let mut last_header = self.get_header(from - 1)?;

        for h in headers.iter() {
            if h.proof_of_stake.timestamp < last_header.proof_of_stake.timestamp {
                return Err(BlockchainError::InvalidTimestamp);
            }

            if h.number != last_header.number + 1 {
                return Err(BlockchainError::InvalidBlockNumber);
            }

            if h.parent_hash != last_header.hash() {
                return Err(BlockchainError::InvalidParentHash);
            }

            last_header = h.clone();
        }

        Ok(true)
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
                chain.apply_block(block)?;
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

        let validator_status = self.validator_status(timestamp, wallet)?;
        if self.config.check_validator && validator_status.is_unproven() {
            return Ok(None);
        }

        let outdated_contracts = self.get_outdated_contracts()?;

        if !outdated_contracts.is_empty() {
            return Err(BlockchainError::StatesOutdated);
        }

        let last_header = self.get_header(height - 1)?;

        let tx_and_deltas = self.select_transactions(mempool, check)?;

        let mut txs = Vec::new();

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
                proof_of_stake: ProofOfStake {
                    timestamp,
                    validator: wallet.get_address(),
                    proof: validator_status,
                },
            },
            body: txs,
        };
        blk.header.block_root = blk.merkle_tree().root();

        match self.isolated(|chain| {
            chain.apply_block(&blk)?; // Check if everything is ok
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
        for (k, v) in self.database.pairs("ACB-".into())?.into_iter() {
            if k.0.ends_with("Ziesha") {
                let bal: Amount = v.try_into().unwrap();
                amount_sum += bal;
            }
        }
        for (k, v) in self.database.pairs("CAB-".into())?.into_iter() {
            if k.0.ends_with("Ziesha") {
                let bal: Amount = v.try_into().unwrap();
                amount_sum += bal;
            }
        }
        for (_, v) in self.database.pairs("DEL-".into())?.into_iter() {
            let bal: Delegate = v.try_into().unwrap();
            amount_sum += bal.amount;
        }
        Ok(amount_sum)
    }

    fn is_validator(
        &self,
        timestamp: u32,
        addr: Address,
        proof: ValidatorProof,
    ) -> Result<bool, BlockchainError> {
        let (epoch, slot) = self.epoch_slot(timestamp);
        let stakers = self.get_stakers()?;
        let sum_stakes = stakers.iter().map(|(_, a)| u64::from(*a)).sum::<u64>();
        let stakers: HashMap<Address, f32> = stakers
            .into_iter()
            .map(|(k, v)| (k, (u64::from(v) as f64 / sum_stakes as f64) as f32))
            .collect();

        if let Some(chance) = stakers.get(&addr) {
            if let Some(staker_info) = self.get_staker(addr.clone())? {
                if let ValidatorProof::Proof {
                    vrf_output,
                    vrf_proof,
                } = proof
                {
                    if Into::<f32>::into(vrf_output.clone()) <= *chance {
                        return Ok(Vrf::verify(
                            &staker_info.vrf_pub_key,
                            format!("{}-{}", epoch, slot).as_bytes(),
                            &vrf_output,
                            &vrf_proof,
                        ));
                    }
                }
            }
        }
        Ok(false)
    }
    fn validator_status(
        &self,
        timestamp: u32,
        wallet: &TxBuilder,
    ) -> Result<ValidatorProof, BlockchainError> {
        let (epoch, slot) = self.epoch_slot(timestamp);
        let stakers = self.get_stakers()?;
        let sum_stakes = stakers.iter().map(|(_, a)| u64::from(*a)).sum::<u64>();
        let stakers: HashMap<Address, f32> = stakers
            .into_iter()
            .map(|(k, v)| (k, (u64::from(v) as f64 / sum_stakes as f64) as f32))
            .collect();
        if let Some(chance) = stakers.get(&wallet.get_address()) {
            let (vrf_output, vrf_proof) = wallet.generate_random(epoch, slot);
            if Into::<f32>::into(vrf_output.clone()) <= *chance {
                Ok(ValidatorProof::Proof {
                    vrf_output,
                    vrf_proof,
                })
            } else {
                Ok(ValidatorProof::Unproven)
            }
        } else {
            Ok(ValidatorProof::Unproven)
        }
    }

    fn cleanup_chain_mempool(
        &self,
        txs: &[ChainSourcedTx],
    ) -> Result<Vec<ChainSourcedTx>, BlockchainError> {
        let (_, result) = self.isolated(|chain| {
            let mut result = Vec::new();
            for tx in txs.iter() {
                match &tx {
                    ChainSourcedTx::TransactionAndDelta(tx_delta) => {
                        if let Err(e) = chain.apply_tx(&tx_delta.tx, false) {
                            log::info!("Rejecting transaction: {}", e);
                        } else {
                            result.push(tx.clone());
                        }
                    }
                    ChainSourcedTx::MpnDeposit(mpn_deposit) => {
                        if let Err(e) = chain.apply_deposit(&mpn_deposit.payment) {
                            log::info!("Rejecting mpn-deposit: {}", e);
                        } else {
                            result.push(tx.clone());
                        }
                    }
                }
            }
            Ok(result)
        })?;
        Ok(result)
    }

    fn get_stake(&self, addr: Address) -> Result<Amount, BlockchainError> {
        Ok(match self.database.get(keys::stake(&addr))? {
            Some(b) => b.try_into()?,
            None => 0.into(),
        })
    }

    fn get_stakers(&self) -> Result<Vec<(Address, Amount)>, BlockchainError> {
        let mut stakers = Vec::new();
        for (k, _) in self
            .database
            .pairs(keys::staker_rank_prefix().into())?
            .into_iter()
        {
            let stake = Amount(
                u64::MAX
                    - u64::from_str_radix(&k.0[4..20], 16)
                        .map_err(|_| BlockchainError::Inconsistency)?,
            );
            let pk: Address = k.0[21..]
                .parse()
                .map_err(|_| BlockchainError::Inconsistency)?;
            if self.get_staker(pk.clone())?.is_some() {
                stakers.push((pk, stake));
            }
        }
        Ok(stakers)
    }

    fn get_delegators(
        &self,
        delegatee: Address,
    ) -> Result<Vec<(Address, Amount)>, BlockchainError> {
        let mut delegators = Vec::new();
        for (k, _) in self
            .database
            .pairs(keys::delegator_rank_prefix(&delegatee).into())?
            .into_iter()
        {
            let splitted = k.0.split("-").collect::<Vec<_>>();
            if splitted.len() != 4 {
                return Err(BlockchainError::Inconsistency);
            }
            let stake = Amount(
                u64::MAX
                    - u64::from_str_radix(&splitted[2], 16)
                        .map_err(|_| BlockchainError::Inconsistency)?,
            );
            let pk: Address = splitted[3]
                .parse()
                .map_err(|_| BlockchainError::Inconsistency)?;
            delegators.push((pk, stake));
        }
        Ok(delegators)
    }

    fn get_delegatees(
        &self,
        delegator: Address,
        top: usize,
    ) -> Result<Vec<(Address, Amount)>, BlockchainError> {
        let mut delegatees = Vec::new();
        for (k, _) in self
            .database
            .pairs(keys::delegatee_rank_prefix(&delegator).into())?
            .into_iter()
        {
            let splitted = k.0.split("-").collect::<Vec<_>>();
            if splitted.len() != 4 {
                return Err(BlockchainError::Inconsistency);
            }
            let stake = Amount(
                u64::MAX
                    - u64::from_str_radix(&splitted[2], 16)
                        .map_err(|_| BlockchainError::Inconsistency)?,
            );
            let pk: Address = splitted[3]
                .parse()
                .map_err(|_| BlockchainError::Inconsistency)?;
            delegatees.push((pk, stake));
            if delegatees.len() >= top {
                break;
            }
        }
        Ok(delegatees)
    }

    fn epoch_slot(&self, timestamp: u32) -> (u32, u32) {
        // TODO: Error instead of saturating_sub!
        let slot_number =
            timestamp.saturating_sub(self.config.chain_start_timestamp) / self.config.slot_duration;
        let epoch_number = slot_number / self.config.slot_per_epoch;
        (epoch_number, slot_number % self.config.slot_per_epoch)
    }

    fn database(&self) -> &K {
        &self.database
    }
}

#[cfg(test)]
mod test;
