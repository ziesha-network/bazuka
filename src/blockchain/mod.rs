mod error;
pub use error::*;

use crate::core::{
    hash::Hash, Account, Address, Block, ChainSourcedTx, ContractAccount, ContractDeposit,
    ContractId, ContractUpdate, ContractWithdraw, Hasher, Header, Money, MpnDeposit, MpnSourcedTx,
    MpnWithdraw, ProofOfWork, RegularSendEntry, Signature, Transaction, TransactionAndDelta,
    TransactionData, ZkHasher as CoreZkHasher,
};
use crate::crypto::jubjub;
use crate::db::{keys, KvStore, RamMirrorKvStore, WriteOp};
use crate::utils;
use crate::wallet::TxBuilder;
use crate::zk;
use crate::zk::ZkHasher;

use rayon::prelude::*;

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::consensus::pow::Difficulty;

#[derive(Clone)]
pub struct BlockchainConfig {
    pub limited_miners: Option<HashSet<Address>>,
    pub genesis: BlockAndPatch,
    pub total_supply: Money,
    pub reward_ratio: u64,
    pub max_block_size: usize,
    pub max_delta_count: usize,
    pub block_time: usize,
    pub difficulty_calc_interval: u64,
    pub pow_base_key: &'static [u8],
    pub pow_key_change_delay: u64,
    pub pow_key_change_interval: u64,
    pub median_timestamp_count: u64,
    pub mpn_contract_id: ContractId,
    pub mpn_num_function_calls: usize,
    pub mpn_num_contract_deposits: usize,
    pub mpn_num_contract_withdraws: usize,
    pub minimum_pow_difficulty: Difficulty,
    pub testnet_height_limit: Option<u64>,
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
    fn currency_in_circulation(&self) -> Result<Money, BlockchainError>;

    fn config(&self) -> &BlockchainConfig;

    fn cleanup_chain_mempool(
        &self,
        mempool: &mut HashMap<ChainSourcedTx, TransactionStats>,
    ) -> Result<(), BlockchainError>;
    fn cleanup_mpn_mempool(
        &self,
        mempool: &mut HashMap<MpnSourcedTx, TransactionStats>,
    ) -> Result<(), BlockchainError>;

    fn db_checksum(&self) -> Result<String, BlockchainError>;

    fn get_account(&self, addr: Address) -> Result<Account, BlockchainError>;
    fn get_mpn_account(&self, index: u32) -> Result<zk::MpnAccount, BlockchainError>;
    fn get_mpn_accounts(
        &self,
        page: usize,
        page_size: usize,
    ) -> Result<Vec<(u32, zk::MpnAccount)>, BlockchainError>;

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
        } else {
            if config.genesis.block != chain.get_block(0)? {
                return Err(BlockchainError::DifferentGenesis);
            }
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
        let height = self.get_height()?;
        let last_block = self.get_tip()?;
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

            let mut contract_account = chain.get_contract_account(deposit.contract_id)?;

            let mut addr_account = chain.get_account(Address::PublicKey(deposit.src.clone()))?;
            if deposit.nonce != addr_account.nonce + 1 {
                return Err(BlockchainError::InvalidTransactionNonce);
            }
            addr_account.nonce += 1;
            if addr_account.balance < deposit.amount + deposit.fee {
                return Err(BlockchainError::BalanceInsufficient);
            }
            addr_account.balance -= deposit.amount + deposit.fee;
            contract_account.balance += deposit.amount;

            chain.database.update(&[WriteOp::Put(
                keys::contract_account(&deposit.contract_id),
                contract_account.clone().into(),
            )])?;
            chain.database.update(&[WriteOp::Put(
                keys::account(&Address::PublicKey(deposit.src.clone())),
                addr_account.into(),
            )])?;
            Ok(())
        })?;
        self.database.update(&ops)?;
        Ok(())
    }

    fn apply_mpn_deposit(&mut self, deposit: &MpnDeposit) -> Result<(), BlockchainError> {
        let (ops, _) = self.isolated(|chain| {
            chain.apply_deposit(&deposit.payment)?;
            let dst = chain.get_mpn_account(deposit.zk_address_index)?;

            let calldata = CoreZkHasher::hash(&[
                zk::ZkScalar::from(deposit.zk_address.decompress().0),
                zk::ZkScalar::from(deposit.zk_address.decompress().1),
            ]);
            if deposit.payment.calldata != calldata {
                return Err(BlockchainError::InvalidMpnTransaction);
            }
            if deposit.zk_address_index > 0x3FFFFFFF {
                return Err(BlockchainError::InvalidMpnTransaction);
            }
            if !deposit.zk_address.is_on_curve() {
                return Err(BlockchainError::InvalidMpnTransaction);
            }
            if dst.address.is_on_curve() && dst.address != deposit.zk_address.decompress() {
                return Err(BlockchainError::InvalidMpnTransaction);
            }
            let mut size_diff = 0;
            zk::KvStoreStateManager::<CoreZkHasher>::set_mpn_account(
                &mut chain.database,
                chain.config.mpn_contract_id,
                deposit.zk_address_index,
                zk::MpnAccount {
                    address: deposit.zk_address.0.decompress(),
                    balance: dst.balance + deposit.payment.amount,
                    nonce: dst.nonce,
                },
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
            let src = chain.get_mpn_account(withdraw.zk_address_index)?;
            if withdraw.zk_address_index > 0x3FFFFFFF {
                return Err(BlockchainError::InvalidMpnTransaction);
            }
            if withdraw.zk_nonce != src.nonce {
                return Err(BlockchainError::InvalidMpnTransaction);
            }
            // TODO: Check signature!
            if src.balance < withdraw.payment.fee + withdraw.payment.amount {
                return Err(BlockchainError::InvalidMpnTransaction);
            }
            let mut size_diff = 0;
            zk::KvStoreStateManager::<CoreZkHasher>::set_mpn_account(
                &mut chain.database,
                chain.config.mpn_contract_id,
                withdraw.zk_address_index,
                zk::MpnAccount {
                    address: src.address.clone(),
                    balance: src.balance - withdraw.payment.fee - withdraw.payment.amount,
                    nonce: src.nonce + 1,
                },
                &mut size_diff,
            )?;
            Ok(())
        })?;
        self.database.update(&ops)?;
        Ok(())
    }

    fn apply_withdraw(&mut self, withdraw: &ContractWithdraw) -> Result<(), BlockchainError> {
        let (ops, _) = self.isolated(|chain| {
            let mut contract_account = chain.get_contract_account(withdraw.contract_id)?;

            let mut addr_account = chain.get_account(Address::PublicKey(withdraw.dst.clone()))?;

            if contract_account.balance < withdraw.amount + withdraw.fee {
                return Err(BlockchainError::ContractBalanceInsufficient);
            }
            contract_account.balance -= withdraw.amount + withdraw.fee;
            addr_account.balance += withdraw.amount;

            chain.database.update(&[WriteOp::Put(
                keys::contract_account(&withdraw.contract_id),
                contract_account.clone().into(),
            )])?;
            chain.database.update(&[WriteOp::Put(
                keys::account(&Address::PublicKey(withdraw.dst.clone())),
                addr_account.into(),
            )])?;
            Ok(())
        })?;
        self.database.update(&ops)?;
        Ok(())
    }

    fn apply_zero_tx(&mut self, tx: &zk::MpnTransaction) -> Result<(), BlockchainError> {
        let (ops, _) = self.isolated(|chain| {
            let src = chain.get_mpn_account(tx.src_index)?;
            let dst = chain.get_mpn_account(tx.dst_index)?;
            if tx.src_index > 0x3FFFFFFF || tx.dst_index > 0x3FFFFFFF {
                return Err(BlockchainError::InvalidMpnTransaction);
            }
            if tx.src_index == tx.dst_index {
                return Err(BlockchainError::InvalidMpnTransaction);
            }
            if !tx.dst_pub_key.is_on_curve() {
                return Err(BlockchainError::InvalidMpnTransaction);
            }
            if dst.address.is_on_curve() && dst.address != tx.dst_pub_key.decompress() {
                return Err(BlockchainError::InvalidMpnTransaction);
            }
            if tx.nonce != src.nonce {
                return Err(BlockchainError::InvalidMpnTransaction);
            }
            if !tx.verify(&jubjub::PublicKey(src.address.compress())) {
                return Err(BlockchainError::InvalidMpnTransaction);
            }
            if src.balance < tx.fee + tx.amount {
                return Err(BlockchainError::InvalidMpnTransaction);
            }
            let mut size_diff = 0;
            zk::KvStoreStateManager::<CoreZkHasher>::set_mpn_account(
                &mut chain.database,
                chain.config.mpn_contract_id,
                tx.src_index,
                zk::MpnAccount {
                    address: src.address.clone(),
                    balance: src.balance - tx.fee - tx.amount,
                    nonce: src.nonce + 1,
                },
                &mut size_diff,
            )?;
            zk::KvStoreStateManager::<CoreZkHasher>::set_mpn_account(
                &mut chain.database,
                chain.config.mpn_contract_id,
                tx.dst_index,
                zk::MpnAccount {
                    address: tx.dst_pub_key.0.decompress(),
                    balance: dst.balance + tx.amount,
                    nonce: dst.nonce,
                },
                &mut size_diff,
            )?;
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
                TransactionData::RegularSend { entries } => {
                    for entry in entries {
                        if entry.dst == tx.src {
                            return Err(BlockchainError::SelfPaymentNotAllowed);
                        }
                        if acc_src.balance < entry.amount {
                            return Err(BlockchainError::BalanceInsufficient);
                        }
                        acc_src.balance -= entry.amount;

                        let mut acc_dst = chain.get_account(entry.dst.clone())?;
                        acc_dst.balance += entry.amount;

                        chain
                            .database
                            .update(&[WriteOp::Put(keys::account(&entry.dst), acc_dst.into())])?;
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
                            balance: Money(0),
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
                    let mut executor_fee = Money(0);

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
                                    if Address::PublicKey(deposit.src.clone()) == tx.src {
                                        return Err(BlockchainError::CannotExecuteOwnPayments);
                                    }
                                    executor_fee += deposit.fee;
                                    state_builder.batch_set(&zk::ZkDeltaPairs(
                                        [
                                            (
                                                zk::ZkDataLocator(vec![i as u32, 0]),
                                                Some(zk::ZkScalar::from(1)),
                                            ),
                                            (
                                                zk::ZkDataLocator(vec![i as u32, 1]),
                                                Some(zk::ZkScalar::from(deposit.amount)),
                                            ),
                                            (
                                                zk::ZkDataLocator(vec![i as u32, 2]),
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
                                            zk::ZkStateModel::Scalar, // Amount
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
                                    if Address::PublicKey(withdraw.dst.clone()) == tx.src {
                                        return Err(BlockchainError::CannotExecuteOwnPayments);
                                    }
                                    executor_fee += withdraw.fee;
                                    state_builder.batch_set(&zk::ZkDeltaPairs(
                                        [
                                            (
                                                zk::ZkDataLocator(vec![i as u32, 0]),
                                                Some(zk::ZkScalar::from(1)),
                                            ),
                                            (
                                                zk::ZkDataLocator(vec![i as u32, 1]),
                                                Some(zk::ZkScalar::from(
                                                    withdraw.amount + withdraw.fee,
                                                )),
                                            ),
                                            (
                                                zk::ZkDataLocator(vec![i as u32, 2]),
                                                Some(fingerprint),
                                            ),
                                            (
                                                zk::ZkDataLocator(vec![i as u32, 3]),
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
                                executor_fee += *fee;

                                let mut cont_account = chain.get_contract_account(*contract_id)?;
                                cont_account.balance -= *fee;
                                chain.database.update(&[WriteOp::Put(
                                    keys::contract_account(contract_id),
                                    cont_account.into(),
                                )])?;

                                let func = contract
                                    .functions
                                    .get(*function_id as usize)
                                    .ok_or(BlockchainError::ContractFunctionNotFound)?;
                                let circuit = &func.verifier_key;
                                let mut state_builder = zk::ZkStateBuilder::<CoreZkHasher>::new(
                                    zk::ZkStateModel::Scalar,
                                );
                                state_builder.batch_set(&zk::ZkDeltaPairs(
                                    [(zk::ZkDataLocator(vec![]), Some(zk::ZkScalar::from(*fee)))]
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
                    acc_src.balance += executor_fee; // Pay executor fee

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
        let mut sorted = txs.iter().cloned().collect::<Vec<_>>();
        sorted.sort_unstable_by_key(|tx| {
            let cost = tx.tx.size();
            let is_mpn = if let TransactionData::UpdateContract { contract_id, .. } = &tx.tx.data {
                *contract_id == self.config.mpn_contract_id
            } else {
                false
            };
            (
                is_mpn,
                Into::<u64>::into(tx.tx.fee) / cost as u64,
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

            if block.header.number > 1850 {
                if block.header.proof_of_work.timestamp < 1669740000 {
                    return Err(BlockchainError::TestnetForcedFork);
                }
            }

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
                let fee_sum = Money(
                    block.body[1..]
                        .iter()
                        .map(|t| -> u64 { t.fee.into() })
                        .sum(),
                );

                let reward_tx = block
                    .body
                    .first()
                    .ok_or(BlockchainError::MinerRewardNotFound)?;

                if reward_tx.src != Address::Treasury
                    || reward_tx.fee != Money(0)
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
                        if reward_entry.amount != next_reward + fee_sum {
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

            if !is_genesis
                && (num_mpn_function_calls < self.config.mpn_num_function_calls
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

    fn get_account(&self, addr: Address) -> Result<Account, BlockchainError> {
        Ok(match self.database.get(keys::account(&addr))? {
            Some(b) => b.try_into()?,
            None => Account {
                balance: if addr == Address::Treasury {
                    self.config.total_supply
                } else {
                    Money(0)
                },
                nonce: 0,
            },
        })
    }

    fn get_mpn_account(&self, index: u32) -> Result<zk::MpnAccount, BlockchainError> {
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
    ) -> Result<Vec<(u32, zk::MpnAccount)>, BlockchainError> {
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
        let mut last_pow = self
            .get_header(
                last_header.number - (last_header.number % self.config.difficulty_calc_interval),
            )?
            .proof_of_work;

        let mut timestamps = (0..std::cmp::min(from, self.config.median_timestamp_count))
            .map(|i| {
                self.get_header(from - 1 - i)
                    .map(|b| b.proof_of_work.timestamp)
            })
            .collect::<Result<Vec<u32>, BlockchainError>>()?;

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

            if h.proof_of_work.timestamp < utils::median(&timestamps) {
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

            timestamps.push(h.proof_of_work.timestamp);
            while timestamps.len() > self.config.median_timestamp_count as usize {
                timestamps.remove(0);
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
    fn next_reward(&self) -> Result<Money, BlockchainError> {
        let supply = self.get_account(Address::Treasury)?.balance;
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
        let treasury_nonce = self.get_account(Address::Treasury)?.nonce;

        let tx_and_deltas = self.select_transactions(mempool, check)?;
        let fee_sum = Money(
            tx_and_deltas
                .iter()
                .map(|t| -> u64 { t.tx.fee.into() })
                .sum(),
        );

        let mut txs = vec![Transaction {
            src: Address::Treasury,
            data: TransactionData::RegularSend {
                entries: vec![RegularSendEntry {
                    dst: wallet.get_address(),
                    amount: self.next_reward()? + fee_sum,
                }],
            },
            nonce: treasury_nonce + 1,
            fee: Money(0),
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

    fn cleanup_chain_mempool(
        &self,
        mempool: &mut HashMap<ChainSourcedTx, TransactionStats>,
    ) -> Result<(), BlockchainError> {
        self.isolated(|chain| {
            let mut txs: Vec<ChainSourcedTx> = mempool.clone().into_keys().collect();
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
                            log::info!("Rejecting transaction: {:?} {}", tx, e);
                            mempool.remove(&tx);
                        }
                    }
                    ChainSourcedTx::MpnDeposit(mpn_deposit) => {
                        if let Err(e) = chain.apply_mpn_deposit(&mpn_deposit) {
                            log::info!("Rejecting transaction: {:?} {}", tx, e);
                            mempool.remove(&tx);
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
        mempool: &mut HashMap<MpnSourcedTx, TransactionStats>,
    ) -> Result<(), BlockchainError> {
        self.isolated(|chain| {
            let mut txs: Vec<MpnSourcedTx> = mempool.clone().into_keys().collect();
            txs.sort_unstable_by_key(|t| t.nonce());
            for tx in txs {
                match &tx {
                    MpnSourcedTx::MpnTransaction(mpn_tx) => {
                        if chain.apply_zero_tx(&mpn_tx).is_err() {
                            mempool.remove(&tx);
                        }
                    }
                    MpnSourcedTx::MpnWithdraw(mpn_withdraw) => {
                        if chain.apply_mpn_withdraw(&mpn_withdraw).is_err() {
                            mempool.remove(&tx);
                        }
                    }
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

    fn currency_in_circulation(&self) -> Result<Money, BlockchainError> {
        let mut money_sum = Money(0);
        for (_, v) in self.database.pairs("ACC-".into())? {
            let acc: Account = v.try_into().unwrap();
            money_sum += acc.balance;
        }
        for (_, v) in self.database.pairs("CAC".into())? {
            let acc: ContractAccount = v.try_into().unwrap();
            money_sum += acc.balance;
        }
        Ok(money_sum)
    }
}

#[cfg(test)]
mod test;
