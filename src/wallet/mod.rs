mod tx_builder;
pub use tx_builder::TxBuilder;

use crate::core::{MpnDeposit, MpnWithdraw, TransactionAndDelta};
use crate::zk::MpnTransaction;
use bip39::Mnemonic;
use rand_core_mnemonic::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WalletError {
    #[error("bincode error happened. wallet corrupted: {0}")]
    BincodeError(#[from] bincode::Error),
    #[error("io error happened: {0}")]
    BlockchainError(#[from] io::Error),
}

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
pub struct Wallet {
    mnemonic: Mnemonic,
    mpn_indices: Vec<u32>,
    rtxs: HashMap<u32, Rtx>, // Nonce -> Tx
    ztxs: HashMap<u64, Ztx>, // Nonce -> Tx
}

impl Wallet {
    pub fn create<R: RngCore + CryptoRng>(rng: &mut R, mnemonic: Option<Mnemonic>) -> Self {
        Self {
            mnemonic: mnemonic.unwrap_or_else(|| {
                Mnemonic::generate_in_with(rng, bip39::Language::English, 12).unwrap()
            }),
            mpn_indices: Default::default(),
            ztxs: Default::default(),
            rtxs: Default::default(),
        }
    }
    pub fn mpn_indices(&self) -> &[u32] {
        &self.mpn_indices
    }
    pub fn add_mpn_index(&mut self, index: u32) {
        self.mpn_indices.push(index);
    }
    pub fn reset(&mut self) {
        self.rtxs.clear();
        self.ztxs.clear();
    }
    pub fn add_rsend(&mut self, tx: TransactionAndDelta) {
        self.rtxs.insert(tx.tx.nonce, Rtx::Rsend(tx));
    }
    pub fn add_deposit(&mut self, tx: MpnDeposit) {
        self.rtxs.insert(tx.payment.nonce, Rtx::Deposit(tx));
    }
    pub fn add_withdraw(&mut self, tx: MpnWithdraw) {
        self.ztxs.insert(tx.zk_nonce, Ztx::Withdraw(tx));
    }
    pub fn add_zsend(&mut self, tx: MpnTransaction) {
        self.ztxs.insert(tx.nonce, Ztx::Zsend(tx));
    }
    pub fn new_r_nonce(&self) -> Option<u32> {
        self.rtxs.keys().max().map(|n| *n + 1)
    }
    pub fn new_z_nonce(&self) -> Option<u64> {
        self.ztxs.keys().max().map(|n| *n + 1)
    }
    pub fn seed(&self) -> [u8; 64] {
        self.mnemonic.to_seed("")
    }
    pub fn mnemonic(&self) -> &Mnemonic {
        &self.mnemonic
    }
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Option<Self>, WalletError> {
        Ok(if let Ok(mut f) = File::open(path) {
            let mut bytes = Vec::new();
            f.read_to_end(&mut bytes)?;
            Some(bincode::deserialize(&bytes)?)
        } else {
            None
        })
    }
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), WalletError> {
        File::create(path)?.write_all(&bincode::serialize(self)?)?;
        Ok(())
    }
}
