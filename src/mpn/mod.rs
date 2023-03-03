pub mod deposit;
pub mod update;
pub mod withdraw;

use crate::blockchain::BlockchainError;
use crate::core::{ContractId, Money, MpnDeposit, MpnWithdraw, TokenId};
use crate::db::{KvStore, WriteOp};
use crate::zk::{groth16::Groth16Proof, MpnAccount, MpnTransaction, ZkDeltaPairs, ZkScalar};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const LOG4_TREE_SIZE: u8 = 15;
pub const LOG4_TOKENS_TREE_SIZE: u8 = 3;
pub const LOG4_DEPOSIT_BATCH_SIZE: u8 = 3;
pub const LOG4_WITHDRAW_BATCH_SIZE: u8 = 3;
pub const LOG4_UPDATE_BATCH_SIZE: u8 = 4;
pub const LOG4_SUPER_UPDATE_BATCH_SIZE: u8 = 5;

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
    config: MpnConfig,
    mpn_contract_id: ContractId,
    mpn_log4_account_capacity: u8,
    db: &K,
    deposits: &[MpnDeposit],
    withdraws: &[MpnWithdraw],
    updates: &[MpnTransaction],
    min_deposit_batch_count: usize,
    min_withdraw_batch_count: usize,
    min_update_batch_count: usize,
) -> Result<(Vec<MpnWork>, ZkDeltaPairs), BlockchainError> {
    let mut mirror = db.mirror();
    let mut works = Vec::new();
    for _ in 0..min_deposit_batch_count {
        let (public_inputs, transitions) = deposit::deposit(
            mpn_contract_id,
            mpn_log4_account_capacity,
            &mut mirror,
            deposits,
        )?;
        works.push(MpnWork {
            config: config.clone(),
            public_inputs,
            data: MpnWorkData::Deposit(transitions),
        });
    }
    for _ in 0..min_withdraw_batch_count {
        let (public_inputs, transitions) = withdraw::withdraw(
            mpn_contract_id,
            mpn_log4_account_capacity,
            &mut mirror,
            withdraws,
        )?;
        works.push(MpnWork {
            config: config.clone(),
            public_inputs,
            data: MpnWorkData::Withdraw(transitions),
        });
    }
    for _ in 0..min_update_batch_count {
        let (public_inputs, transitions) = update::update(
            mpn_contract_id,
            mpn_log4_account_capacity,
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
    let delta = extract_delta(&ops);
    Ok((works, delta))
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
