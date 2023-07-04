pub mod circuits;
pub mod deposit;
pub mod update;
pub mod withdraw;

use crate::blockchain::{Blockchain, BlockchainError};
use crate::core::{
    hash::Hash, Address, Amount, ContractId, ContractUpdate, ContractUpdateData, Hasher, Money,
    MpnAddress, MpnDeposit, MpnWithdraw, Signature, TokenId, Transaction, TransactionAndDelta,
    TransactionData,
};
use crate::db::{KvStore, KvStoreError, WriteOp};
use crate::wallet::TxBuilder;
use crate::zk::{
    check_proof, MpnAccount, MpnTransaction, ZkCompressedState, ZkDeltaPairs, ZkProof, ZkScalar,
    ZkStateModel, ZkVerifierKey,
};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MpnError {
    #[error("blockchain error happened: {0}")]
    BlockchainError(#[from] BlockchainError),
    #[error("kv-store error happened: {0}")]
    KvStoreError(#[from] KvStoreError),
    #[error("insufficient workers in the pool")]
    InsufficientWorkers,
}

fn extract_delta(ops: &[WriteOp]) -> ZkDeltaPairs {
    let mut pairs = ZkDeltaPairs([].into());
    for op in ops {
        match op {
            WriteOp::Put(k, v) => {
                let mut it = k.0.split("-S-");
                it.next();
                if let Some(loc) = it.next() {
                    pairs
                        .0
                        .insert(loc.parse().unwrap(), Some(v.clone().try_into().unwrap()));
                }
            }
            WriteOp::Remove(k) => {
                let mut it = k.0.split("-S-");
                it.next();
                if let Some(loc) = it.next() {
                    pairs.0.insert(loc.parse().unwrap(), None);
                }
            }
        }
    }
    pairs
}

pub struct MpnSolution {
    prover: Address,
    proof: ZkProof,
}

pub struct MpnWorkPool {
    config: MpnConfig,
    final_delta: ZkDeltaPairs,
    works: HashMap<usize, MpnWork>,
    solutions: HashMap<usize, MpnSolution>,
    assignees: HashMap<Address, Vec<usize>>,
}

impl MpnWorkPool {
    pub fn remaining_works(&self) -> HashMap<usize, MpnWork> {
        let mut remaining = self.works.clone();
        for solved in self.solutions.keys() {
            remaining.remove(solved);
        }
        remaining
    }
    pub fn get_works(&mut self, address: Address) -> HashMap<usize, MpnWork> {
        let selected = if let Some(works) = self.assignees.get(&address) {
            works.clone()
        } else {
            let mut not_assigned: HashMap<usize, MpnWork> =
                self.remaining_works().into_iter().collect();
            for wid in self.assignees.values().flatten() {
                not_assigned.remove(wid);
            }

            let selected = not_assigned
                .keys()
                .cloned()
                .choose_multiple(&mut rand::thread_rng(), 2);
            self.assignees.insert(address, selected.clone());
            selected
        };

        selected
            .iter()
            .filter_map(|i| {
                if let Some(w) = self.works.get(i) {
                    Some((*i, w.clone()))
                } else {
                    None
                }
            })
            .collect()
    }
    pub fn prove(&mut self, id: usize, prover: &Address, proof: &ZkProof) -> bool {
        if !self.solutions.contains_key(&id) {
            if let Some(work) = self.works.get(&id) {
                if work.verify(prover, proof) {
                    self.solutions.insert(
                        id,
                        MpnSolution {
                            prover: prover.clone(),
                            proof: proof.clone(),
                        },
                    );
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    }
    pub fn ready(&self, tx_builder: &TxBuilder, nonce: u32) -> Option<TransactionAndDelta> {
        if self.works.len() == self.solutions.len() {
            let mut updates = vec![];
            for i in 0..self.works.len() {
                updates.push(match self.works[&i].data.clone() {
                    MpnWorkData::Deposit(trans) => ContractUpdate {
                        data: ContractUpdateData::Deposit {
                            deposits: trans.into_iter().map(|t| t.tx.payment.clone()).collect(),
                        },
                        circuit_id: 0,
                        next_state: self.works[&i].new_root.clone(),
                        proof: self.solutions[&i].proof.clone(),
                        reward: self.works[&i].reward,
                        prover: self.solutions[&i].prover.clone(),
                    },
                    MpnWorkData::Withdraw(trans) => ContractUpdate {
                        data: ContractUpdateData::Withdraw {
                            withdraws: trans.into_iter().map(|t| t.tx.payment.clone()).collect(),
                        },
                        circuit_id: 0,
                        next_state: self.works[&i].new_root.clone(),
                        proof: self.solutions[&i].proof.clone(),
                        reward: self.works[&i].reward,
                        prover: self.solutions[&i].prover.clone(),
                    },
                    MpnWorkData::Update(trans) => {
                        assert!(trans.iter().all(|t| t.tx.fee.token_id == TokenId::Ziesha));
                        let fee_sum = trans
                            .iter()
                            .map(|t| Into::<u64>::into(t.tx.fee.amount))
                            .sum::<u64>();
                        ContractUpdate {
                            data: ContractUpdateData::FunctionCall {
                                fee: Money {
                                    token_id: TokenId::Ziesha,
                                    amount: fee_sum.into(),
                                },
                            },
                            circuit_id: 0,
                            next_state: self.works[&i].new_root.clone(),
                            proof: self.solutions[&i].proof.clone(),
                            reward: self.works[&i].reward,
                            prover: self.solutions[&i].prover.clone(),
                        }
                    }
                });
            }
            let mut update = Transaction {
                memo: String::new(),
                src: Some(tx_builder.get_address()),
                nonce: nonce,
                fee: Money::ziesha(0),
                data: TransactionData::UpdateContract {
                    contract_id: self.config.mpn_contract_id.clone(),
                    updates,
                    delta: Some(self.final_delta.clone()),
                },
                sig: Signature::Unsigned,
            };
            tx_builder.sign_tx(&mut update);
            Some(TransactionAndDelta {
                tx: update,
                state_delta: Some(self.final_delta.clone()),
            })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpnConfig {
    pub log4_tree_size: u8,
    pub log4_token_tree_size: u8,
    pub log4_deposit_batch_size: u8,
    pub log4_withdraw_batch_size: u8,
    pub log4_update_batch_size: u8,
    pub mpn_contract_id: ContractId,
    pub mpn_num_update_batches: usize,
    pub mpn_num_deposit_batches: usize,
    pub mpn_num_withdraw_batches: usize,
    pub deposit_vk: ZkVerifierKey,
    pub withdraw_vk: ZkVerifierKey,
    pub update_vk: ZkVerifierKey,
}

impl MpnConfig {
    pub fn state_model(&self) -> ZkStateModel {
        ZkStateModel::List {
            log4_size: self.log4_tree_size,
            item_type: Box::new(ZkStateModel::Struct {
                field_types: vec![
                    ZkStateModel::Scalar, // Tx-Nonce
                    ZkStateModel::Scalar, // Withdraw-Nonce
                    ZkStateModel::Scalar, // Pub-key X
                    ZkStateModel::Scalar, // Pub-key Y
                    ZkStateModel::List {
                        log4_size: self.log4_token_tree_size,
                        item_type: Box::new(ZkStateModel::Struct {
                            field_types: vec![
                                ZkStateModel::Scalar, // Token-Id
                                ZkStateModel::Scalar, // Balance
                            ],
                        }),
                    },
                ],
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MpnWorkData {
    Deposit(Vec<DepositTransition>),
    Withdraw(Vec<WithdrawTransition>),
    Update(Vec<UpdateTransition>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkPublicInputs {
    pub height: u64,
    pub state: ZkScalar,
    pub aux_data: ZkScalar,
    pub next_state: ZkScalar,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpnWorker {
    pub address: Address,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpnWork {
    pub config: MpnConfig,
    pub public_inputs: ZkPublicInputs,
    pub data: MpnWorkData,
    pub new_root: ZkCompressedState,
    pub reward: Amount,
}

impl MpnWork {
    pub fn vk(&self) -> ZkVerifierKey {
        match &self.data {
            MpnWorkData::Deposit(_) => &self.config.deposit_vk,
            MpnWorkData::Withdraw(_) => &self.config.withdraw_vk,
            MpnWorkData::Update(_) => &self.config.update_vk,
        }
        .clone()
    }
    pub fn verify(&self, prover: &Address, proof: &ZkProof) -> bool {
        let vk = self.vk();
        let commitment = ZkScalar::new(
            Hasher::hash(&bincode::serialize(&(prover.clone(), self.reward)).unwrap()).as_ref(),
        );
        check_proof(
            &vk,
            commitment,
            self.public_inputs.height,
            self.public_inputs.state,
            self.public_inputs.aux_data,
            self.public_inputs.next_state,
            proof,
        )
    }
}

pub fn prepare_works<K: KvStore, B: Blockchain<K>>(
    config: &MpnConfig,
    db: &B,
    workers: &HashMap<Address, MpnWorker>,
    mut deposits: Vec<MpnDeposit>,
    withdraws: Vec<MpnWithdraw>,
    updates: Vec<MpnTransaction>,
    block_reward: Amount,
    deposit_reward: Amount,
    withdraw_reward: Amount,
    update_reward: Amount,
    validator_tx_builder_deposit_nonce: u32,
    validator_tx_builder: TxBuilder,
    user_tx_builder: TxBuilder,
) -> Result<MpnWorkPool, MpnError> {
    let mut mirror = db.fork_on_ram();
    let mut works = Vec::new();
    let mut workers = workers.values().cloned().collect::<Vec<_>>();
    if workers.len() == 0 {
        log::warn!("No MPN-workers defined! All proving rewards will go into validator's wallet!");
        workers = vec![MpnWorker {
            address: user_tx_builder.get_address(),
        }];
    }
    let mut worker_id = 0;
    let mut new_account_indices = HashMap::<MpnAddress, u64>::new();

    let remaining_reward: Amount = (u64::from(block_reward)
        - config.mpn_num_deposit_batches as u64 * u64::from(deposit_reward)
        - config.mpn_num_withdraw_batches as u64 * u64::from(withdraw_reward)
        - config.mpn_num_update_batches as u64 * u64::from(update_reward))
    .into();

    let validator_balance = db.get_balance(validator_tx_builder.get_address(), TokenId::Ziesha)?;
    mirror.database_mut().update(&[WriteOp::Put(
        crate::db::keys::account_balance(&validator_tx_builder.get_address(), TokenId::Ziesha),
        (validator_balance + remaining_reward).into(),
    )])?;

    deposits.insert(
        0,
        validator_tx_builder.deposit_mpn(
            "".into(),
            config.mpn_contract_id,
            validator_tx_builder.get_mpn_address(),
            validator_tx_builder_deposit_nonce + 1,
            Money {
                token_id: TokenId::Ziesha,
                amount: remaining_reward,
            },
            Money::ziesha(0),
        ),
    );

    for _ in 0..config.mpn_num_deposit_batches {
        let (new_root, public_inputs, transitions) = deposit::deposit(
            config.mpn_contract_id,
            config.log4_tree_size,
            config.log4_token_tree_size,
            config.log4_deposit_batch_size,
            &mut mirror,
            &deposits,
            &mut new_account_indices,
            true,
        )?;
        log::info!("Made MPN-Deposit block of {} txs.", transitions.len());
        works.push(MpnWork {
            config: config.clone(),
            public_inputs,
            new_root,
            data: MpnWorkData::Deposit(transitions),
            reward: deposit_reward,
        });
        worker_id = (worker_id + 1) % workers.len();
    }
    for _ in 0..config.mpn_num_withdraw_batches {
        let (new_root, public_inputs, transitions) = withdraw::withdraw(
            config.mpn_contract_id,
            config.log4_tree_size,
            config.log4_token_tree_size,
            config.log4_withdraw_batch_size,
            &mut mirror,
            &withdraws,
            &mut new_account_indices,
        )?;
        log::info!("Made MPN-Withdraw block of {} txs.", transitions.len());
        works.push(MpnWork {
            config: config.clone(),
            public_inputs,
            new_root,
            data: MpnWorkData::Withdraw(transitions),
            reward: withdraw_reward,
        });
        worker_id = (worker_id + 1) % workers.len();
    }
    for _ in 0..config.mpn_num_update_batches {
        let (new_root, public_inputs, transitions) = update::update(
            config.mpn_contract_id,
            config.log4_tree_size,
            config.log4_token_tree_size,
            config.log4_update_batch_size,
            TokenId::Ziesha,
            &mut mirror,
            &updates,
            &mut new_account_indices,
        )?;
        log::info!("Made MPN-Update block of {} txs.", transitions.len());
        works.push(MpnWork {
            config: config.clone(),
            public_inputs,
            new_root,
            data: MpnWorkData::Update(transitions),
            reward: update_reward,
        });
        worker_id = (worker_id + 1) % workers.len();
    }
    let ops = mirror.database().to_ops();
    let final_delta = extract_delta(&ops);
    Ok(MpnWorkPool {
        config: config.clone(),
        works: works.into_iter().enumerate().collect(),
        final_delta,
        solutions: HashMap::new(),
        assignees: HashMap::new(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositTransition {
    pub enabled: bool,
    pub tx: MpnDeposit,
    pub before: MpnAccount,
    pub before_balances_hash: ZkScalar,
    pub before_balance: Money,
    pub proof: Vec<[ZkScalar; 3]>,
    pub account_index: u64,
    pub token_index: u64,
    pub balance_proof: Vec<[ZkScalar; 3]>,
}

impl DepositTransition {
    pub fn null(log4_tree_size: u8, log4_token_tree_size: u8) -> Self {
        Self {
            enabled: false,
            tx: Default::default(),
            before: Default::default(),
            before_balances_hash: Default::default(),
            before_balance: Default::default(),
            proof: vec![Default::default(); log4_tree_size as usize],
            account_index: Default::default(),
            token_index: Default::default(),
            balance_proof: vec![Default::default(); log4_token_tree_size as usize],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawTransition {
    pub enabled: bool,
    pub tx: MpnWithdraw,
    pub before: MpnAccount,
    pub before_token_balance: Money,
    pub before_fee_balance: Money,
    pub proof: Vec<[ZkScalar; 3]>,
    pub account_index: u64,
    pub token_index: u64,
    pub token_balance_proof: Vec<[ZkScalar; 3]>,
    pub before_token_hash: ZkScalar,
    pub fee_token_index: u64,
    pub fee_balance_proof: Vec<[ZkScalar; 3]>,
}

impl WithdrawTransition {
    pub fn null(log4_tree_size: u8, log4_token_tree_size: u8) -> Self {
        Self {
            enabled: false,
            tx: Default::default(),
            before: Default::default(),
            before_token_balance: Default::default(),
            before_fee_balance: Default::default(),
            account_index: Default::default(),
            proof: vec![Default::default(); log4_tree_size as usize],
            token_index: Default::default(),
            token_balance_proof: vec![Default::default(); log4_token_tree_size as usize],
            before_token_hash: Default::default(),
            fee_token_index: Default::default(),
            fee_balance_proof: vec![Default::default(); log4_token_tree_size as usize],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTransition {
    pub enabled: bool,
    pub tx: MpnTransaction,
    pub src_before: MpnAccount,
    pub src_before_balances_hash: ZkScalar,
    pub src_before_balance: Money,
    pub src_before_fee_balance: Money,
    pub src_proof: Vec<[ZkScalar; 3]>,
    pub src_index: u64,
    pub src_token_index: u64,
    pub src_balance_proof: Vec<[ZkScalar; 3]>,
    pub src_fee_token_index: u64,
    pub src_fee_balance_proof: Vec<[ZkScalar; 3]>,
    pub dst_before: MpnAccount,
    pub dst_before_balances_hash: ZkScalar,
    pub dst_before_balance: Money,
    pub dst_proof: Vec<[ZkScalar; 3]>,
    pub dst_index: u64,
    pub dst_token_index: u64,
    pub dst_balance_proof: Vec<[ZkScalar; 3]>,
}

impl UpdateTransition {
    pub fn null(log4_tree_size: u8, log4_token_tree_size: u8) -> Self {
        Self {
            enabled: false,
            tx: Default::default(),
            src_before: Default::default(),
            src_before_balances_hash: Default::default(),
            src_before_balance: Default::default(),
            src_before_fee_balance: Default::default(),
            src_index: Default::default(),
            src_proof: vec![Default::default(); log4_tree_size as usize],
            src_token_index: Default::default(),
            src_balance_proof: vec![Default::default(); log4_token_tree_size as usize],
            src_fee_token_index: Default::default(),
            src_fee_balance_proof: vec![Default::default(); log4_token_tree_size as usize],
            dst_before: Default::default(),
            dst_before_balances_hash: Default::default(),
            dst_before_balance: Default::default(),
            dst_index: Default::default(),
            dst_proof: vec![Default::default(); log4_tree_size as usize],
            dst_token_index: Default::default(),
            dst_balance_proof: vec![Default::default(); log4_token_tree_size as usize],
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::blockchain::BlockchainConfig;
    use crate::blockchain::KvStoreChain;
    use crate::db::RamKvStore;

    pub fn fresh_db(conf: BlockchainConfig) -> KvStoreChain<RamKvStore> {
        KvStoreChain::new(RamKvStore::new(), conf).unwrap()
    }
}
