pub mod deposit;
pub mod update;
pub mod withdraw;

use crate::core::{Money, MpnDeposit, MpnWithdraw};
use crate::zk::{MpnAccount, MpnTransaction, ZkScalar};
use serde::{Deserialize, Serialize};

pub const LOG4_TREE_SIZE: u8 = 15;
pub const LOG4_TOKENS_TREE_SIZE: u8 = 3;
pub const LOG4_DEPOSIT_BATCH_SIZE: u8 = 3;
pub const LOG4_WITHDRAW_BATCH_SIZE: u8 = 3;
pub const LOG4_UPDATE_BATCH_SIZE: u8 = 4;
pub const LOG4_SUPER_UPDATE_BATCH_SIZE: u8 = 5;

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
pub struct MpnWork {
    pub config: MpnConfig,
    pub data: MpnWorkData,
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
