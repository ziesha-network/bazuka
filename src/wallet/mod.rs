mod tx_builder;
pub use tx_builder::TxBuilder;

use crate::core::{
    ChainSourcedTx, MpnAddress, MpnDeposit, MpnSourcedTx, MpnWithdraw, TokenId, TransactionAndDelta,
};
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
pub struct Wallet {
    mnemonic: Mnemonic,
    pub tokens: Vec<TokenId>,
    pub chain_sourced_txs: Vec<ChainSourcedTx>,
    pub mpn_sourced_txs: HashMap<MpnAddress, Vec<MpnSourcedTx>>,
}

impl Wallet {
    pub fn create<R: RngCore + CryptoRng>(rng: &mut R, mnemonic: Option<Mnemonic>) -> Self {
        Self {
            mnemonic: mnemonic.unwrap_or_else(|| {
                Mnemonic::generate_in_with(rng, bip39::Language::English, 12).unwrap()
            }),
            chain_sourced_txs: Vec::new(),
            mpn_sourced_txs: HashMap::new(),
            tokens: vec![TokenId::Ziesha],
        }
    }
    pub fn add_token(&mut self, token_id: TokenId) {
        if !self.tokens.contains(&token_id) {
            self.tokens.push(token_id);
        }
    }
    pub fn get_tokens(&self) -> &[TokenId] {
        &self.tokens
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
            .entry(MpnAddress {
                pub_key: tx.zk_address.clone(),
            })
            .or_default()
            .push(MpnSourcedTx::MpnWithdraw(tx));
    }
    pub fn add_zsend(&mut self, tx: MpnTransaction) {
        self.mpn_sourced_txs
            .entry(MpnAddress {
                pub_key: tx.src_pub_key.clone(),
            })
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
    pub fn new_z_nonce(&self, addr: &MpnAddress) -> Option<u64> {
        if let Some(Some(n)) = self
            .mpn_sourced_txs
            .get(addr)
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
            Ok(Some(bincode::deserialize(&bytes)?))
        } else {
            Ok(None)
        }
    }
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), WalletError> {
        File::create(path)?.write_all(&bincode::serialize(self)?)?;
        Ok(())
    }
}
