mod tx_builder;
pub use tx_builder::TxBuilder;

use crate::core::{MpnDeposit, MpnWithdraw, TransactionAndDelta};
use crate::zk::MpnTransaction;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Rtx {
    Rsend(TransactionAndDelta),
    Deposit(MpnDeposit),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Ztx {
    Zsend(MpnTransaction),
    Withdraw(MpnWithdraw),
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WalletData {
    seed: Vec<u8>,
    mpn_indices: Vec<u32>,
    rtxs: HashMap<u32, Rtx>, // Nonce -> Tx
    ztxs: HashMap<u64, Ztx>, // Nonce -> Tx
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct Wallet {
    builder: TxBuilder,
}

#[allow(dead_code)]
impl Wallet {
    pub fn new(seed: &[u8]) -> Self {
        Self {
            builder: TxBuilder::new(seed.to_vec()),
        }
    }
}
