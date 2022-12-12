mod tx_builder;
pub use tx_builder::TxBuilder;

use crate::core::{ChainSourcedTx, MpnDeposit, MpnSourcedTx, MpnWithdraw, TransactionAndDelta};
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
pub struct LegacyWallet {
    mnemonic: Mnemonic,
    tx_nonce: Option<u32>,
    mpn_nonces: HashMap<u32, Option<u64>>, // Nonce for each MPN account
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Wallet {
    mnemonic: Mnemonic,
    pub chain_sourced_txs: Vec<ChainSourcedTx>,
    pub mpn_sourced_txs: HashMap<u32, Vec<MpnSourcedTx>>,
}

impl Wallet {
    pub fn create<R: RngCore + CryptoRng>(rng: &mut R, mnemonic: Option<Mnemonic>) -> Self {
        Self {
            mnemonic: mnemonic.unwrap_or_else(|| {
                Mnemonic::generate_in_with(rng, bip39::Language::English, 12).unwrap()
            }),
            chain_sourced_txs: Vec::new(),
            mpn_sourced_txs: HashMap::new(),
        }
    }
    pub fn mpn_indices(&self) -> Vec<u32> {
        self.mpn_sourced_txs.keys().cloned().collect()
    }
    pub fn add_mpn_index(&mut self, index: u32) {
        self.mpn_sourced_txs.insert(index, Vec::new());
    }
    pub fn reset(&mut self) {
        self.chain_sourced_txs = Vec::new();
        self.mpn_sourced_txs.iter_mut().for_each(|(_, v)| {
            *v = Vec::new();
        });
    }
    pub fn add_rsend(&mut self, tx: TransactionAndDelta) {
        self.chain_sourced_txs
            .push(ChainSourcedTx::TransactionAndDelta(tx));
    }
    pub fn add_deposit(&mut self, tx: MpnDeposit) {
        self.chain_sourced_txs.push(ChainSourcedTx::MpnDeposit(tx));
    }
    pub fn add_withdraw(&mut self, tx: MpnWithdraw) {
        self.mpn_sourced_txs
            .entry(tx.zk_address_index)
            .or_default()
            .push(MpnSourcedTx::MpnWithdraw(tx));
    }
    pub fn add_zsend(&mut self, tx: MpnTransaction) {
        self.mpn_sourced_txs
            .entry(tx.src_index)
            .or_default()
            .push(MpnSourcedTx::MpnTransaction(tx));
    }
    pub fn new_r_nonce(&self) -> Option<u32> {
        self.chain_sourced_txs
            .iter()
            .map(|tx| tx.nonce())
            .max()
            .map(|n| n + 1)
    }
    pub fn new_z_nonce(&self, index: u32) -> Option<u64> {
        if let Some(Some(n)) = self
            .mpn_sourced_txs
            .get(&index)
            .map(|it| it.iter().map(|tx| tx.nonce()).max())
        {
            Some(n + 1)
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
        if let Ok(mut f) = File::open(&path) {
            let mut bytes = Vec::new();
            f.read_to_end(&mut bytes)?;
            let wallet: Result<Self, bincode::Error> = bincode::deserialize(&bytes);
            match wallet {
                Ok(w) => Ok(Some(w)),
                Err(e) => {
                    let legacy: Option<LegacyWallet> = bincode::deserialize(&bytes).ok();
                    if let Some(legacy) = legacy {
                        println!("Migrating wallet...");
                        let wallet = Self {
                            mnemonic: legacy.mnemonic,
                            chain_sourced_txs: Vec::new(),
                            mpn_sourced_txs: HashMap::new(),
                        };
                        wallet.save(&path)?;
                        Ok(Some(wallet))
                    } else {
                        Err(WalletError::BincodeError(e))
                    }
                }
            }
        } else {
            Ok(None)
        }
    }
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), WalletError> {
        File::create(path)?.write_all(&bincode::serialize(self)?)?;
        Ok(())
    }
}
