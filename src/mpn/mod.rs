pub mod deposit;
pub mod update;
pub mod withdraw;

use crate::blockchain::BlockchainError;
use crate::core::{
    ContractId, ContractUpdate, Money, MpnAddress, MpnDeposit, MpnWithdraw, Signature, TokenId,
    Transaction, TransactionAndDelta, TransactionData,
};
use crate::db::{KvStore, WriteOp};
use crate::wallet::TxBuilder;
use crate::zk::{
    groth16::groth16_verify, groth16::Groth16Proof, groth16::Groth16VerifyingKey, MpnAccount,
    MpnTransaction, ZkCompressedState, ZkDeltaPairs, ZkProof, ZkScalar,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MpnError {
    #[error("blockchain error happened: {0}")]
    BlockchainError(#[from] BlockchainError),
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

pub struct MpnWorkPool {
    config: MpnConfig,
    final_delta: ZkDeltaPairs,
    works: HashMap<usize, MpnWork>,
    solutions: HashMap<usize, Groth16Proof>,
}

impl MpnWorkPool {
    pub fn get_works(&self, token: String) -> HashMap<usize, MpnWork> {
        let mut remaining = self.works.clone();
        for solved in self.solutions.keys() {
            remaining.remove(solved);
        }
        remaining
            .into_iter()
            .filter(|(_, v)| v.worker.token == token)
            .collect()
    }
    pub fn prove(&mut self, id: usize, proof: &Groth16Proof) -> bool {
        if !self.solutions.contains_key(&id) {
            if let Some(work) = self.works.get(&id) {
                if work.verify(proof) {
                    self.solutions.insert(id, proof.clone());
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
                    MpnWorkData::Deposit(trans) => ContractUpdate::Deposit {
                        deposit_circuit_id: 0,
                        deposits: trans.into_iter().map(|t| t.tx.payment.clone()).collect(),
                        next_state: self.works[&i].new_root.clone(),
                        proof: ZkProof::Groth16(Box::new(self.solutions[&i].clone())),
                    },
                    MpnWorkData::Withdraw(trans) => ContractUpdate::Withdraw {
                        withdraw_circuit_id: 0,
                        withdraws: trans.into_iter().map(|t| t.tx.payment.clone()).collect(),
                        next_state: self.works[&i].new_root.clone(),
                        proof: ZkProof::Groth16(Box::new(self.solutions[&i].clone())),
                    },
                    MpnWorkData::Update(_) => ContractUpdate::FunctionCall {
                        function_id: 0,
                        next_state: self.works[&i].new_root.clone(),
                        proof: ZkProof::Groth16(Box::new(self.solutions[&i].clone())),
                        fee: Money {
                            token_id: TokenId::Ziesha,
                            amount: 0.into(),
                        },
                    },
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
    pub deposit_vk: Groth16VerifyingKey,
    pub withdraw_vk: Groth16VerifyingKey,
    pub update_vk: Groth16VerifyingKey,
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
    pub token: String,
    pub mpn_address: MpnAddress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpnWork {
    pub config: MpnConfig,
    pub public_inputs: ZkPublicInputs,
    pub data: MpnWorkData,
    pub new_root: ZkCompressedState,
    pub worker: MpnWorker,
}

impl MpnWork {
    pub fn vk(&self) -> Groth16VerifyingKey {
        match &self.data {
            MpnWorkData::Deposit(_) => &self.config.deposit_vk,
            MpnWorkData::Withdraw(_) => &self.config.withdraw_vk,
            MpnWorkData::Update(_) => &self.config.update_vk,
        }
        .clone()
    }
    pub fn verify(&self, proof: &Groth16Proof) -> bool {
        let vk = self.vk();
        groth16_verify(
            &vk,
            self.public_inputs.height,
            self.public_inputs.state,
            self.public_inputs.aux_data,
            self.public_inputs.next_state,
            proof,
        )
    }
}

pub fn prepare_works<K: KvStore>(
    config: &MpnConfig,
    db: &K,
    workers: &HashMap<String, MpnWorker>,
    deposits: &[MpnDeposit],
    withdraws: &[MpnWithdraw],
    updates: &[MpnTransaction],
) -> Result<MpnWorkPool, MpnError> {
    let mut mirror = db.mirror();
    let mut works = Vec::new();
    let workers = workers.values().collect::<Vec<_>>();
    if workers.len() == 0 {
        return Err(MpnError::InsufficientWorkers);
    }
    let mut worker_id = 0;

    for _ in 0..config.mpn_num_deposit_batches {
        let (new_root, public_inputs, transitions) = deposit::deposit(
            config.mpn_contract_id,
            config.log4_tree_size,
            config.log4_token_tree_size,
            config.log4_deposit_batch_size,
            &mut mirror,
            deposits,
        )?;
        works.push(MpnWork {
            config: config.clone(),
            public_inputs,
            new_root,
            data: MpnWorkData::Deposit(transitions),
            worker: workers[worker_id].clone(),
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
            withdraws,
        )?;
        works.push(MpnWork {
            config: config.clone(),
            public_inputs,
            new_root,
            data: MpnWorkData::Withdraw(transitions),
            worker: workers[worker_id].clone(),
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
            updates,
        )?;
        works.push(MpnWork {
            config: config.clone(),
            public_inputs,
            new_root,
            data: MpnWorkData::Update(transitions),
            worker: workers[worker_id].clone(),
        });
        worker_id = (worker_id + 1) % workers.len();
    }
    let ops = mirror.to_ops();
    let final_delta = extract_delta(&ops);
    Ok(MpnWorkPool {
        config: config.clone(),
        works: works.into_iter().enumerate().collect(),
        final_delta,
        solutions: HashMap::new(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositTransition {
    pub tx: MpnDeposit,
    pub before: MpnAccount,
    pub before_balances_hash: ZkScalar,
    pub before_balance: Money,
    pub proof: Vec<[ZkScalar; 3]>,
    pub balance_proof: Vec<[ZkScalar; 3]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawTransition {
    pub tx: MpnWithdraw,
    pub before: MpnAccount,
    pub before_token_balance: Money,
    pub before_fee_balance: Money,
    pub proof: Vec<[ZkScalar; 3]>,
    pub token_balance_proof: Vec<[ZkScalar; 3]>,
    pub before_token_hash: ZkScalar,
    pub fee_balance_proof: Vec<[ZkScalar; 3]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTransition {
    pub tx: MpnTransaction,
    pub src_before: MpnAccount,
    pub src_before_balances_hash: ZkScalar,
    pub src_before_balance: Money,
    pub src_before_fee_balance: Money,
    pub src_proof: Vec<[ZkScalar; 3]>,
    pub src_balance_proof: Vec<[ZkScalar; 3]>,
    pub src_fee_balance_proof: Vec<[ZkScalar; 3]>,
    pub dst_before: MpnAccount,
    pub dst_before_balances_hash: ZkScalar,
    pub dst_before_balance: Money,
    pub dst_proof: Vec<[ZkScalar; 3]>,
    pub dst_balance_proof: Vec<[ZkScalar; 3]>,
}
