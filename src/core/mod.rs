mod address;
mod blocks;
pub mod hash;
mod header;
mod money;
mod transaction;

use crate::crypto;
use crate::zk;

use std::str::FromStr;
use thiserror::Error;

pub use money::Amount;
pub use transaction::Money;

pub type Hasher = hash::Sha3Hasher;
pub type Signer = crypto::ed25519::Ed25519<Hasher>;

pub type ZkHasher = crate::zk::PoseidonHasher;
pub type ZkSigner = crypto::jubjub::JubJub<ZkHasher>;

pub type Address = <Signer as crypto::SignatureScheme>::Pub;
pub type ParseAddressError = <Signer as crypto::SignatureScheme>::PubParseError;
pub type Account = address::Account;
pub type Delegate = address::Delegate;
pub type Signature = address::Signature<Signer>;
pub type Transaction = transaction::Transaction<Hasher, Signer>;
pub type TransactionData = transaction::TransactionData<Hasher, Signer>;
pub type RegularSendEntry = transaction::RegularSendEntry<Signer>;
pub type ContractAccount = transaction::ContractAccount;
pub type ContractUpdate = transaction::ContractUpdate<Hasher, Signer>;
pub type ContractDeposit = transaction::ContractDeposit<Hasher, Signer>;
pub type ContractWithdraw = transaction::ContractWithdraw<Hasher, Signer>;
pub type MpnAddress = address::MpnAddress<ZkSigner>;
pub type ParseMpnAddressError = address::ParseMpnAddressError;
pub type MpnDeposit = transaction::MpnDeposit<Hasher, Signer, ZkSigner>;
pub type MpnWithdraw = transaction::MpnWithdraw<Hasher, Signer, ZkSigner>;
pub type MpnTransaction = zk::MpnTransaction;
pub type Header = header::Header<Hasher>;
pub type Block = blocks::Block<Hasher, Signer>;
pub type TokenId = transaction::TokenId;
pub type ParseTokenIdError = transaction::ParseTokenIdError;
pub type TokenUpdate = transaction::TokenUpdate<Signer>;
pub type Token = transaction::Token<Signer>;

pub type ProofOfWork = header::ProofOfWork;
pub type ContractId = transaction::ContractId<Hasher>;

pub type TransactionAndDelta = transaction::TransactionAndDelta<Hasher, Signer>;

#[derive(Error, Debug)]
pub enum ParseZieshaAddressError {
    #[error("ziesha address invalid")]
    Invalid,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum ZieshaAddress {
    ChainAddress(Address),
    MpnAddress(MpnAddress),
}

impl std::fmt::Display for ZieshaAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::ChainAddress(addr) => write!(f, "{}", addr),
            Self::MpnAddress(addr) => write!(f, "{}", addr),
        }
    }
}

impl FromStr for ZieshaAddress {
    type Err = ParseZieshaAddressError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok::<Address, ParseAddressError>(addr) = s.parse() {
            Ok(Self::ChainAddress(addr))
        } else if let Ok::<MpnAddress, ParseMpnAddressError>(addr) = s.parse() {
            Ok(Self::MpnAddress(addr))
        } else {
            Err(Self::Err::Invalid)
        }
    }
}

// Transactions initiated from chain accounts
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum ChainSourcedTx {
    TransactionAndDelta(TransactionAndDelta),
    MpnDeposit(MpnDeposit),
}

impl ChainSourcedTx {
    pub fn sender(&self) -> Address {
        match self {
            ChainSourcedTx::TransactionAndDelta(tx_delta) => {
                tx_delta.tx.src.clone().unwrap_or_default()
            }
            ChainSourcedTx::MpnDeposit(mpn_deposit) => mpn_deposit.payment.src.clone(),
        }
    }
    pub fn nonce(&self) -> u32 {
        match self {
            ChainSourcedTx::TransactionAndDelta(tx_delta) => tx_delta.tx.nonce,
            ChainSourcedTx::MpnDeposit(mpn_deposit) => mpn_deposit.payment.nonce,
        }
    }
    pub fn verify_signature(&self) -> bool {
        match self {
            ChainSourcedTx::TransactionAndDelta(tx_delta) => tx_delta.tx.verify_signature(),
            ChainSourcedTx::MpnDeposit(mpn_deposit) => mpn_deposit.payment.verify_signature(),
        }
    }
}

// Transactions initiated from MPN accounts
#[allow(clippy::large_enum_variant)]
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub enum MpnSourcedTx {
    MpnTransaction(MpnTransaction),
    MpnWithdraw(MpnWithdraw),
}

impl MpnSourcedTx {
    pub fn sender(&self) -> MpnAddress {
        match self {
            MpnSourcedTx::MpnTransaction(mpn_tx) => MpnAddress {
                pub_key: mpn_tx.src_pub_key.clone(),
            },
            MpnSourcedTx::MpnWithdraw(mpn_withdraw) => MpnAddress {
                pub_key: mpn_withdraw.zk_address.clone(),
            },
        }
    }
    pub fn nonce(&self) -> u64 {
        match self {
            MpnSourcedTx::MpnTransaction(mpn_tx) => mpn_tx.nonce,
            MpnSourcedTx::MpnWithdraw(mpn_withdraw) => mpn_withdraw.zk_nonce,
        }
    }
    pub fn verify_signature(&self) -> bool {
        match self {
            MpnSourcedTx::MpnTransaction(mpn_tx) => mpn_tx.verify_signature(),
            MpnSourcedTx::MpnWithdraw(mpn_withdraw) => mpn_withdraw.verify_signature::<ZkHasher>(),
        }
    }
}

impl PartialEq<ChainSourcedTx> for ChainSourcedTx {
    fn eq(&self, other: &Self) -> bool {
        bincode::serialize(self).unwrap() == bincode::serialize(other).unwrap()
    }
}
impl Eq for ChainSourcedTx {}
impl std::hash::Hash for ChainSourcedTx {
    fn hash<Hasher>(&self, state: &mut Hasher)
    where
        Hasher: std::hash::Hasher,
    {
        state.write(&bincode::serialize(self).unwrap());
        state.finish();
    }
}

impl PartialEq<MpnSourcedTx> for MpnSourcedTx {
    fn eq(&self, other: &Self) -> bool {
        bincode::serialize(self).unwrap() == bincode::serialize(other).unwrap()
    }
}
impl Eq for MpnSourcedTx {}
impl std::hash::Hash for MpnSourcedTx {
    fn hash<Hasher>(&self, state: &mut Hasher)
    where
        Hasher: std::hash::Hasher,
    {
        state.write(&bincode::serialize(self).unwrap());
        state.finish();
    }
}
