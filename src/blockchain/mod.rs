mod error;
pub use error::*;
mod mempool;
pub use mempool::*;
mod config;
pub use config::BlockchainConfig;
mod ops;

use crate::core::{
    hash::Hash, Address, Amount, Block, ContractAccount, ContractDeposit, ContractId,
    ContractUpdate, ContractWithdraw, Delegate, Hasher, Header, Money, MpnAddress, ProofOfStake,
    Ratio, RegularSendEntry, Signature, Staker, Token, TokenId, TokenUpdate, Transaction,
    TransactionAndDelta, TransactionData, Undelegation, UndelegationId, ValidatorProof, Vrf,
    ZkHasher as CoreZkHasher,
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ZkCompressedStateChange {
    prev_state: zk::ZkCompressedState,
    state: zk::ZkCompressedState,
    prev_height: u64,
}

pub trait Blockchain<K: KvStore> {
    fn fork_on_ram(&self) -> KvStoreChain<RamMirrorKvStore<'_, K>>;
    fn epoch_randomness(&self) -> Result<<Hasher as Hash>::Output, BlockchainError>;
    fn database(&self) -> &K;
    fn database_mut(&mut self) -> &mut K;
    fn epoch_slot(&self, timestamp: u32) -> (u32, u32);
    fn get_mpn_account_count(&self) -> Result<u64, BlockchainError>;
    fn get_mpn_account_indices(&self, addr: MpnAddress) -> Result<Vec<u64>, BlockchainError>;
    fn get_stake(&self, addr: Address) -> Result<Amount, BlockchainError>;
    fn get_stakers(&self) -> Result<Vec<(Address, Amount)>, BlockchainError>;
    fn get_auto_delegate_ratio(
        &self,
        delegator: Address,
        delegatee: Address,
    ) -> Result<Ratio, BlockchainError>;
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
    fn get_undelegation(
        &self,
        undelegator: Address,
        undelegation_id: UndelegationId,
    ) -> Result<Option<Undelegation>, BlockchainError>;
    fn get_undelegations(
        &self,
        undelegator: Address,
        top: Option<usize>,
    ) -> Result<Vec<(UndelegationId, Undelegation)>, BlockchainError>;
    fn get_staker(&self, addr: Address) -> Result<Option<Staker>, BlockchainError>;
    fn get_nonce(&self, addr: Address) -> Result<u32, BlockchainError>;
    fn get_mpn_account(&self, addr: MpnAddress) -> Result<zk::MpnAccount, BlockchainError>;
    fn get_mpn_accounts(
        &self,
        page: usize,
        page_size: usize,
    ) -> Result<Vec<(u64, zk::MpnAccount)>, BlockchainError>;

    fn get_deposit_nonce(
        &self,
        addr: Address,
        contract_id: ContractId,
    ) -> Result<u32, BlockchainError>;

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
    ) -> Result<Option<Block>, BlockchainError>;
    fn get_height(&self) -> Result<u64, BlockchainError>;
    fn get_tip(&self) -> Result<Header, BlockchainError>;
    fn get_headers(&self, since: u64, count: u64) -> Result<Vec<Header>, BlockchainError>;
    fn get_blocks(&self, since: u64, count: u64) -> Result<Vec<Block>, BlockchainError>;
    fn get_header(&self, index: u64) -> Result<Header, BlockchainError>;
    fn get_block(&self, index: u64) -> Result<Block, BlockchainError>;

    fn get_contract(&self, contract_id: ContractId) -> Result<zk::ZkContract, BlockchainError>;

    fn check_tx(&self, tx: &Transaction) -> Result<(), BlockchainError>;
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
            chain.apply_block(&config.genesis)?;
        } else if config.genesis != chain.get_block(0)? {
            return Err(BlockchainError::DifferentGenesis);
        }

        Ok(chain)
    }

    fn isolated<F, R>(&self, f: F) -> Result<(Vec<WriteOp>, R), BlockchainError>
    where
        F: FnOnce(&mut KvStoreChain<RamMirrorKvStore<'_, K>>) -> Result<R, BlockchainError>,
    {
        let mut mirror = self.fork_on_ram();
        let result = f(&mut mirror)?;
        Ok((mirror.database.to_ops(), result))
    }

    fn apply_deposit(&mut self, deposit: &ContractDeposit) -> Result<(), BlockchainError> {
        ops::apply_deposit(self, deposit)
    }

    fn apply_withdraw(&mut self, withdraw: &ContractWithdraw) -> Result<(), BlockchainError> {
        ops::apply_withdraw(self, withdraw)
    }

    fn apply_tx(&mut self, tx: &Transaction, internal: bool) -> Result<(), BlockchainError> {
        ops::apply_tx(self, tx, internal)
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

    fn get_tip(&self) -> Result<Header, BlockchainError> {
        let height = self.get_height()?;
        if height == 0 {
            Err(BlockchainError::BlockchainEmpty)
        } else {
            self.get_header(height - 1)
        }
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

    fn get_nonce(&self, addr: Address) -> Result<u32, BlockchainError> {
        Ok(match self.database.get(keys::nonce(&addr))? {
            Some(b) => b.try_into()?,
            None => 0,
        })
    }

    fn get_deposit_nonce(
        &self,
        addr: Address,
        contract_id: ContractId,
    ) -> Result<u32, BlockchainError> {
        Ok(
            match self
                .database
                .get(keys::deposit_nonce(&addr, &contract_id))?
            {
                Some(b) => b.try_into()?,
                None => 0,
            },
        )
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

    fn get_mpn_account(&self, addr: MpnAddress) -> Result<zk::MpnAccount, BlockchainError> {
        let index = if let Some(ind) = self.get_mpn_account_indices(addr.clone())?.first() {
            *ind
        } else {
            return Ok(Default::default());
        };
        let acc = zk::KvStoreStateManager::<CoreZkHasher>::get_mpn_account(
            &self.database,
            self.config.mpn_config.mpn_contract_id,
            index,
        )?;
        if acc.address.is_on_curve() && acc.address != addr.pub_key.0.decompress() {
            return Err(BlockchainError::Inconsistency);
        }
        Ok(acc)
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
            let (last_epoch, last_slot) = self.epoch_slot(last_header.proof_of_stake.timestamp);
            let (h_epoch, h_slot) = self.epoch_slot(h.proof_of_stake.timestamp);

            if [h_epoch, h_slot] <= [last_epoch, last_slot] {
                return Err(BlockchainError::InvalidEpochSlot);
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
    ) -> Result<Option<Block>, BlockchainError> {
        ops::draft_block(self, timestamp, mempool, wallet, check)
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
        for (_, v) in self.database.pairs("UDL-".into())?.into_iter() {
            let bal: Undelegation = v.try_into().unwrap();
            amount_sum += bal.amount;
        }
        Ok(amount_sum)
    }

    fn epoch_randomness(&self) -> Result<<Hasher as Hash>::Output, BlockchainError> {
        Ok(match self.database.get(keys::randomness())? {
            Some(b) => b.try_into()?,
            None => Default::default(),
        })
    }

    fn is_validator(
        &self,
        timestamp: u32,
        addr: Address,
        proof: ValidatorProof,
    ) -> Result<bool, BlockchainError> {
        let (epoch, slot) = self.epoch_slot(timestamp);
        let randomness = self.epoch_randomness()?;
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
                        let preimage = format!("{}-{}-{}", hex::encode(randomness), epoch, slot);

                        return Ok(Vrf::verify(
                            &staker_info.vrf_pub_key,
                            preimage.as_bytes(),
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
        let randomness = self.epoch_randomness()?;
        let stakers = self.get_stakers()?;
        let sum_stakes = stakers.iter().map(|(_, a)| u64::from(*a)).sum::<u64>();
        let stakers: HashMap<Address, f32> = stakers
            .into_iter()
            .map(|(k, v)| (k, (u64::from(v) as f64 / sum_stakes as f64) as f32))
            .collect();
        if let Some(chance) = stakers.get(&wallet.get_address()) {
            let (vrf_output, vrf_proof) = wallet.generate_random(randomness, epoch, slot);
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
            .pairs(keys::StakerRankDbKey::prefix().into())?
            .into_iter()
        {
            let staker_rank = keys::StakerRankDbKey::try_from(k)?;
            if self.get_staker(staker_rank.address.clone())?.is_some() {
                stakers.push((staker_rank.address, staker_rank.amount));
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
            .pairs(keys::DelegatorRankDbKey::prefix(&delegatee).into())?
            .into_iter()
        {
            let delegator_rank = keys::DelegatorRankDbKey::try_from(k)?;
            delegators.push((delegator_rank.delegator, delegator_rank.amount));
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
            .pairs(keys::DelegateeRankDbKey::prefix(&delegator).into())?
            .into_iter()
        {
            let delegatee_rank = keys::DelegateeRankDbKey::try_from(k)?;
            delegatees.push((delegatee_rank.delegatee, delegatee_rank.amount));
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
    fn database_mut(&mut self) -> &mut K {
        &mut self.database
    }
    fn min_validator_reward(&self, validator: Address) -> Result<Amount, BlockchainError> {
        let (_, result) =
            self.isolated(|chain| Ok(chain.pay_validator_and_delegators(validator, Amount(0))?))?;
        Ok(result)
    }
    fn check_tx(&self, tx: &Transaction) -> Result<(), BlockchainError> {
        let mut chain = self.fork_on_ram();
        chain.apply_tx(&tx, false)?;

        Ok(())
    }
    fn get_auto_delegate_ratio(
        &self,
        delegator: Address,
        delegatee: Address,
    ) -> Result<Ratio, BlockchainError> {
        Ok(
            match self
                .database
                .get(keys::auto_delegate(&delegator, &delegatee))?
            {
                Some(b) => b.try_into()?,
                None => Ratio(0),
            },
        )
    }
    fn get_undelegation(
        &self,
        undelegator: Address,
        undelegation_id: UndelegationId,
    ) -> Result<Option<Undelegation>, BlockchainError> {
        Ok(
            match self.database.get(
                keys::UndelegationDbKey {
                    undelegator,
                    undelegation_id,
                }
                .into(),
            )? {
                Some(b) => Some(b.try_into()?),
                None => None,
            },
        )
    }
    fn get_undelegations(
        &self,
        undelegator: Address,
        top: Option<usize>,
    ) -> Result<Vec<(UndelegationId, Undelegation)>, BlockchainError> {
        let mut undelegations = Vec::new();
        for (k, v) in self
            .database
            .pairs(keys::UndelegationDbKey::prefix(&undelegator).into())?
            .into_iter()
        {
            let undelegation_id = keys::UndelegationDbKey::try_from(k)?.undelegation_id;
            let undelegation: Undelegation = v.try_into()?;
            undelegations.push((undelegation_id, undelegation));
            if let Some(top) = top {
                if undelegations.len() >= top {
                    break;
                }
            }
        }
        Ok(undelegations)
    }
    fn get_mpn_account_count(&self) -> Result<u64, BlockchainError> {
        Ok(match self.database.get(keys::mpn_account_count())? {
            Some(b) => b.try_into()?,
            None => 0,
        })
    }
    fn get_mpn_account_indices(&self, addr: MpnAddress) -> Result<Vec<u64>, BlockchainError> {
        let mut indices = Vec::new();
        for (k, _) in self
            .database
            .pairs(keys::MpnAccountIndexDbKey::prefix(&addr).into())?
            .into_iter()
        {
            indices.push(keys::MpnAccountIndexDbKey::try_from(k)?.index);
        }
        Ok(indices)
    }

    fn fork_on_ram(&self) -> KvStoreChain<RamMirrorKvStore<'_, K>> {
        KvStoreChain {
            database: self.database.mirror(),
            config: self.config.clone(),
        }
    }
}

#[cfg(test)]
mod test;
