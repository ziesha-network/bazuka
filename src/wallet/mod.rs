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
    pub fn mnemonic(&self) -> &Mnemonic {
        &self.mnemonic
    }
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, WalletError> {
        let mut bytes = Vec::new();
        File::open(path)?.read_to_end(&mut bytes)?;
        Ok(bincode::deserialize(&bytes)?)
    }
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), WalletError> {
        File::open(path)?.write_all(&bincode::serialize(self)?)?;
        Ok(())
    }
}
