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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WalletType {
    User(usize),
    Validator,
}

impl WalletType {
    fn bip39_passphrase(&self) -> String {
        match self {
            WalletType::User(index) => {
                if *index == 0 {
                    "".into()
                } else {
                    index.to_string()
                }
            }
            WalletType::Validator => "validator".into(),
        }
    }
}

#[derive(Error, Debug)]
pub enum WalletError {
    #[error("bincode error happened. wallet corrupted: {0}")]
    BincodeError(#[from] bincode::Error),
    #[error("io error happened: {0}")]
    BlockchainError(#[from] io::Error),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WalletCollection {
    mnemonic: Mnemonic,
    wallets: HashMap<WalletType, Wallet>,
}

impl WalletCollection {
    pub fn mnemonic(&self) -> &Mnemonic {
        &self.mnemonic
    }
    pub fn create<R: RngCore + CryptoRng>(rng: &mut R, mnemonic: Option<Mnemonic>) -> Self {
        Self {
            mnemonic: mnemonic.unwrap_or_else(|| {
                Mnemonic::generate_in_with(rng, bip39::Language::English, 12).unwrap()
            }),
            wallets: Default::default(),
        }
    }
    fn seed(&self, wallet_type: WalletType) -> [u8; 64] {
        self.mnemonic.to_seed(&wallet_type.bip39_passphrase())
    }
    pub fn user_builder(&self, index: usize) -> TxBuilder {
        TxBuilder::new(&self.seed(WalletType::User(index)))
    }
    pub fn validator_builder(&self) -> TxBuilder {
        TxBuilder::new(&self.seed(WalletType::Validator))
    }
    pub fn user(&mut self, index: usize) -> &mut Wallet {
        self.wallets.entry(WalletType::User(index)).or_default()
    }
    pub fn validator(&mut self) -> &mut Wallet {
        self.wallets.entry(WalletType::Validator).or_default()
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Wallet {
    pub tokens: Vec<TokenId>,
    pub chain_sourced_txs: Vec<ChainSourcedTx>,
    pub mpn_sourced_txs: HashMap<MpnAddress, Vec<MpnSourcedTx>>,
}

impl Default for Wallet {
    fn default() -> Self {
        Self {
            chain_sourced_txs: Vec::new(),
            mpn_sourced_txs: HashMap::new(),
            tokens: vec![TokenId::Ziesha],
        }
    }
}

impl Wallet {
    pub fn delete_chain_tx(&mut self, nonce: u32, resigner: TxBuilder) {
        self.chain_sourced_txs.retain(|t| t.nonce() != nonce);
        for tx in self.chain_sourced_txs.iter_mut() {
            if tx.nonce() > nonce {
                match tx {
                    ChainSourcedTx::TransactionAndDelta(tx_delta) => {
                        tx_delta.tx.nonce -= 1;
                        resigner.sign_tx(&mut tx_delta.tx);
                    }
                    ChainSourcedTx::MpnDeposit(mpn_deposit) => {
                        mpn_deposit.payment.nonce -= 1;
                        resigner.sign_deposit(&mut mpn_deposit.payment);
                    }
                }
            }
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
}
