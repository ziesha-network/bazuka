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

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Wallet {
    mnemonic: Mnemonic,
    tx_nonce: Option<u32>,
    mpn_nonces: HashMap<u32, Option<u64>>, // Nonce for each MPN account
}

impl Wallet {
    pub fn create<R: RngCore + CryptoRng>(rng: &mut R, mnemonic: Option<Mnemonic>) -> Self {
        Self {
            mnemonic: mnemonic.unwrap_or_else(|| {
                Mnemonic::generate_in_with(rng, bip39::Language::English, 12).unwrap()
            }),
            tx_nonce: None,
            mpn_nonces: HashMap::new(),
        }
    }
    pub fn mpn_indices(&self) -> Vec<u32> {
        self.mpn_nonces.keys().cloned().collect()
    }
    pub fn add_mpn_index(&mut self, index: u32) {
        self.mpn_nonces.insert(index, None);
    }
    pub fn reset(&mut self) {
        self.tx_nonce = None;
        self.mpn_nonces.iter_mut().for_each(|(_, v)| {
            *v = None;
        });
    }
    pub fn add_rsend(&mut self, tx: TransactionAndDelta) {
        self.tx_nonce = Some(tx.tx.nonce);
    }
    pub fn add_deposit(&mut self, tx: MpnDeposit) {
        self.tx_nonce = Some(tx.payment.nonce);
    }
    pub fn add_withdraw(&mut self, tx: MpnWithdraw) {
        self.mpn_nonces
            .insert(tx.zk_address_index, Some(tx.zk_nonce));
    }
    pub fn add_zsend(&mut self, tx: MpnTransaction) {
        self.mpn_nonces.insert(tx.src_index, Some(tx.nonce));
    }
    pub fn new_r_nonce(&self) -> Option<u32> {
        self.tx_nonce.map(|n| n + 1)
    }
    pub fn new_z_nonce(&self, index: u32) -> Option<u64> {
        if let Some(Some(n)) = self.mpn_nonces.get(&index).map(|n| n.map(|n| n + 1)) {
            Some(n)
        } else {
            None
        }
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
