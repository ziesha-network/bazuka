mod tx_builder;
pub use tx_builder::TxBuilder;

use crate::core::{ContractId, GeneralTransaction, NonceGroup};

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
    pub fn user(&mut self, index: usize) -> &mut Wallet {
        self.wallets
            .entry(WalletType::User(index))
            .or_insert(Wallet::new(WalletType::User(index), self.mnemonic.clone()))
    }
    pub fn validator(&mut self) -> &mut Wallet {
        self.wallets
            .entry(WalletType::Validator)
            .or_insert(Wallet::new(WalletType::Validator, self.mnemonic.clone()))
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
    pub mnemonic: Mnemonic,
    pub wallet_type: WalletType,
    pub tokens: Vec<ContractId>,
    pub txs: HashMap<NonceGroup, Vec<GeneralTransaction>>,
}

impl Wallet {
    pub fn new(wallet_type: WalletType, mnemonic: Mnemonic) -> Self {
        Self {
            txs: HashMap::new(),
            tokens: vec![ContractId::Ziesha],
            wallet_type,
            mnemonic,
        }
    }
    fn seed(&self) -> [u8; 64] {
        self.mnemonic.to_seed(&self.wallet_type.bip39_passphrase())
    }
    pub fn tx_builder(&self) -> TxBuilder {
        TxBuilder::new(&self.seed())
    }
    pub fn add_token(&mut self, token_id: ContractId) {
        if !self.tokens.contains(&token_id) {
            self.tokens.push(token_id);
        }
    }
    pub fn get_tokens(&self) -> &[ContractId] {
        &self.tokens
    }
    pub fn reset(&mut self) {
        self.txs.iter_mut().for_each(|(_, v)| {
            *v = Vec::new();
        });
    }
    pub fn add_tx(&mut self, tx: GeneralTransaction) {
        self.txs.entry(tx.nonce_group()).or_default().push(tx);
    }
    pub fn new_nonce(&self, addr: NonceGroup) -> Option<u32> {
        if let Some(Some(n)) = self
            .txs
            .get(&addr)
            .map(|it| it.iter().map(|tx| tx.nonce()).max())
        {
            Some(n + 1)
        } else {
            None
        }
    }
}
