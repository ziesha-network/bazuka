mod tx_builder;
pub use tx_builder::TxBuilder;

use crate::core::{MpnPayment, TransactionAndDelta};
use crate::zk::MpnTransaction;
use std::collections::HashMap;

pub struct WalletData {
    seed: Vec<u8>,
    tx: HashMap<u32, TransactionAndDelta>, // Nonce -> Tx
    mpn_indices: Vec<u32>,
    tx_zk: HashMap<u32, HashMap<u32, MpnPayment>>, // Account -> Nonce -> Tx
    zk: HashMap<u32, HashMap<u64, MpnTransaction>>, // Account -> Nonce -> Tx
}

#[derive(Clone)]
pub struct Wallet {
    builder: TxBuilder,
}

impl Wallet {
    pub fn new(seed: &[u8]) -> Self {
        Self {
            builder: TxBuilder::new(seed.to_vec()),
        }
    }
}
