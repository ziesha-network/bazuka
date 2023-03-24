mod error;
pub use error::*;
mod mempool;
pub use mempool::*;
mod config;
pub use config::BlockchainConfig;
mod ops;

use crate::core::{
    hash::Hash, Account, Address, Amount, Block, ChainSourcedTx, ContractAccount, ContractDeposit,
    ContractId, ContractUpdate, ContractWithdraw, Delegate, Hasher, Header, Money, ProofOfStake,
    RegularSendEntry, Signature, Staker, Token, TokenId, TokenUpdate, Transaction,
    TransactionAndDelta, TransactionData, ValidatorProof, Vrf, ZkHasher as CoreZkHasher,
};
use crate::crypto::VerifiableRandomFunction;
use crate::db::{keys, KvStore, RamMirrorKvStore, WriteOp};

use crate::wallet::TxBuilder;
use crate::zk;

use rayon::prelude::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TransactionValidity {
    Unknown,
    Invalid,
    Valid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
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
        top: Option<usize>,
    ) -> Result<Vec<(Address, Amount)>, BlockchainError>;
    fn get_delegators(
        &self,
        delegatee: Address,
        top: Option<usize>,
    ) -> Result<Vec<(Address, Amount)>, BlockchainError>;
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
    fn min_validator_reward(&self, validator: Address) -> Result<Amount, BlockchainError>;

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
        ops::apply_deposit(self, deposit)
    }

    fn apply_withdraw(&mut self, withdraw: &ContractWithdraw) -> Result<(), BlockchainError> {
        ops::apply_withdraw(self, withdraw)
    }

    fn apply_tx(
        &mut self,
        tx: &Transaction,
        allow_treasury: bool,
    ) -> Result<TxSideEffect, BlockchainError> {
        ops::apply_tx(self, tx, allow_treasury)
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

    fn pay_validator_and_delegators(
        &mut self,
        validator: Address,
        fee_sum: Amount,
    ) -> Result<Amount, BlockchainError> {
        ops::pay_validator_and_delegators(self, validator, fee_sum)
    }

    fn select_transactions(
        &self,
        validator: Address,
        txs: &[TransactionAndDelta],
        check: bool,
    ) -> Result<Vec<TransactionAndDelta>, BlockchainError> {
        ops::select_transactions(self, validator, txs, check)
    }

    fn apply_block(&mut self, block: &Block) -> Result<(), BlockchainError> {
        ops::apply_block(self, block)
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
        ops::rollback(self)
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
        ops::draft_block(self, timestamp, mempool, wallet, check)
    }

    fn update_states(&mut self, patch: &ZkBlockchainPatch) -> Result<(), BlockchainError> {
        ops::update_states(self, patch)
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
        ops::generate_state_patch(self, heights, to)
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
        top: Option<usize>,
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
            if let Some(top) = top {
                if delegators.len() >= top {
                    break;
                }
            }
        }
        Ok(delegators)
    }

    fn get_delegatees(
        &self,
        delegator: Address,
        top: Option<usize>,
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
            if let Some(top) = top {
                if delegatees.len() >= top {
                    break;
                }
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
    fn min_validator_reward(&self, validator: Address) -> Result<Amount, BlockchainError> {
        let (_, result) =
            self.isolated(|chain| Ok(chain.pay_validator_and_delegators(validator, Amount(0))?))?;
        Ok(result)
    }
}

#[cfg(test)]
mod test;
