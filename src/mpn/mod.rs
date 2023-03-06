pub mod deposit;
pub mod update;
pub mod withdraw;

use crate::blockchain::BlockchainError;
use crate::core::{ContractId, Money, MpnDeposit, MpnWithdraw, TokenId};
use crate::db::{KvStore, WriteOp};
use crate::zk::{groth16::Groth16Proof, MpnAccount, MpnTransaction, ZkDeltaPairs, ZkScalar};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    final_delta: ZkDeltaPairs,
    works: HashMap<usize, MpnWork>,
    solutions: HashMap<usize, Groth16Proof>,
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
pub struct MpnWork {
    pub config: MpnConfig,
    pub public_inputs: ZkPublicInputs,
    pub data: MpnWorkData,
}

pub fn prepare_works<K: KvStore>(
    config: &MpnConfig,
    db: &K,
    deposits: &[MpnDeposit],
    withdraws: &[MpnWithdraw],
    updates: &[MpnTransaction],
) -> Result<MpnWorkPool, BlockchainError> {
    let mut mirror = db.mirror();
    let mut works = Vec::new();
    for _ in 0..config.mpn_num_deposit_batches {
        let (public_inputs, transitions) = deposit::deposit(
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
            data: MpnWorkData::Deposit(transitions),
        });
    }
    for _ in 0..config.mpn_num_withdraw_batches {
        let (public_inputs, transitions) = withdraw::withdraw(
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
            data: MpnWorkData::Withdraw(transitions),
        });
    }
    for _ in 0..config.mpn_num_update_batches {
        let (public_inputs, transitions) = update::update(
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
            data: MpnWorkData::Update(transitions),
        });
    }
    let ops = mirror.to_ops();
    let final_delta = extract_delta(&ops);
    Ok(MpnWorkPool {
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
