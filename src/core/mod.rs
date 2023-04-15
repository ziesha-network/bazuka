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
pub use money::Decimal;
pub use transaction::{Money, Ratio};

pub type Hasher = hash::Sha3Hasher;
pub type Signer = crypto::ed25519::Ed25519<Hasher>;
pub type Vrf = crypto::vrf::VRF;

pub type ZkHasher = crate::zk::PoseidonHasher;
pub type ZkSigner = crypto::jubjub::JubJub<ZkHasher>;

pub type ConvertRatioError = transaction::ConvertRatioError;

pub type Address = <Signer as crypto::SignatureScheme>::Pub;
pub type ParseAddressError = <Signer as crypto::SignatureScheme>::PubParseError;
pub type Staker = address::Staker<Vrf>;
pub type Delegate = address::Delegate;
pub type Undelegation = address::Undelegation;
pub type Signature = address::Signature<Signer>;
pub type Transaction = transaction::Transaction<Hasher, Signer, Vrf>;
pub type TransactionData = transaction::TransactionData<Hasher, Signer, Vrf>;
pub type RegularSendEntry = transaction::RegularSendEntry<Signer>;
pub type ContractAccount = transaction::ContractAccount;
pub type ContractUpdate = transaction::ContractUpdate<Hasher, Signer>;
pub type ContractDeposit = transaction::ContractDeposit<Hasher, Signer>;
pub type ContractWithdraw = transaction::ContractWithdraw<Hasher, Signer>;
pub type MpnAddress = address::MpnAddress<ZkSigner>;
pub type ParseMpnAddressError = address::ParseMpnAddressError;
pub type UndelegationId = transaction::UndelegationId<Hasher>;
pub type ParseUndelegationIdError = transaction::ParseUndelegationIdError;
pub type MpnDeposit = transaction::MpnDeposit<Hasher, Signer, ZkSigner>;
pub type MpnWithdraw = transaction::MpnWithdraw<Hasher, Signer, ZkSigner>;
pub type MpnTransaction = zk::MpnTransaction;
pub type Header = header::Header<Hasher, Signer, Vrf>;
pub type ValidatorProof = header::ValidatorProof<Vrf>;
pub type Block = blocks::Block<Hasher, Signer, Vrf>;
pub type TokenId = transaction::TokenId;
pub type ParseTokenIdError = transaction::ParseTokenIdError;
pub type TokenUpdate = transaction::TokenUpdate<Signer>;
pub type Token = transaction::Token<Signer>;

pub type ProofOfStake = header::ProofOfStake<Signer, Vrf>;
pub type ContractId = transaction::ContractId<Hasher>;

pub type TransactionAndDelta = transaction::TransactionAndDelta<Hasher, Signer, Vrf>;

#[derive(Error, Debug)]
pub enum ParseGeneralAddressError {
    #[error("ziesha address invalid")]
    Invalid,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GeneralAddress {
    ChainAddress(Address),
    MpnAddress(MpnAddress),
}

impl std::fmt::Display for GeneralAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::ChainAddress(addr) => write!(f, "{}", addr),
            Self::MpnAddress(addr) => write!(f, "{}", addr),
        }
    }
}

impl FromStr for GeneralAddress {
    type Err = ParseGeneralAddressError;
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

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum NonceGroup {
    TransactionAndDelta(Address),
    MpnDeposit(Address),
    MpnTransaction(MpnAddress),
    MpnWithdraw(MpnAddress),
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum GeneralTransaction {
    TransactionAndDelta(TransactionAndDelta),
    MpnDeposit(MpnDeposit),
    MpnTransaction(MpnTransaction),
    MpnWithdraw(MpnWithdraw),
}

impl GeneralTransaction {
    pub fn nonce_group(&self) -> NonceGroup {
        match self {
            GeneralTransaction::TransactionAndDelta(tx_delta) => {
                NonceGroup::TransactionAndDelta(tx_delta.tx.src.clone().unwrap_or_default())
            }
            GeneralTransaction::MpnDeposit(mpn_deposit) => {
                NonceGroup::MpnDeposit(mpn_deposit.payment.src.clone())
            }
            GeneralTransaction::MpnTransaction(mpn_tx) => NonceGroup::MpnTransaction(MpnAddress {
                pub_key: mpn_tx.src_pub_key.clone(),
            }),
            GeneralTransaction::MpnWithdraw(mpn_withdraw) => NonceGroup::MpnWithdraw(MpnAddress {
                pub_key: mpn_withdraw.zk_address.clone(),
            }),
        }
    }
    pub fn verify_signature(&self) -> bool {
        match self {
            GeneralTransaction::TransactionAndDelta(tx_delta) => tx_delta.tx.verify_signature(),
            GeneralTransaction::MpnDeposit(mpn_deposit) => mpn_deposit.payment.verify_signature(),
            GeneralTransaction::MpnTransaction(mpn_tx) => mpn_tx.verify_signature(),
            GeneralTransaction::MpnWithdraw(mpn_withdraw) => {
                mpn_withdraw.verify_signature::<ZkHasher>()
            }
        }
    }
    pub fn nonce(&self) -> u32 {
        match self {
            GeneralTransaction::TransactionAndDelta(tx_delta) => tx_delta.tx.nonce,
            GeneralTransaction::MpnDeposit(mpn_deposit) => mpn_deposit.payment.nonce,
            GeneralTransaction::MpnTransaction(mpn_tx) => mpn_tx.nonce,
            GeneralTransaction::MpnWithdraw(mpn_withdraw) => mpn_withdraw.zk_nonce,
        }
    }
    pub fn sender(&self) -> GeneralAddress {
        match self {
            GeneralTransaction::TransactionAndDelta(tx_delta) => {
                GeneralAddress::ChainAddress(tx_delta.tx.src.clone().unwrap_or_default())
            }
            GeneralTransaction::MpnDeposit(mpn_deposit) => {
                GeneralAddress::ChainAddress(mpn_deposit.payment.src.clone())
            }
            GeneralTransaction::MpnTransaction(mpn_tx) => GeneralAddress::MpnAddress(MpnAddress {
                pub_key: mpn_tx.src_pub_key.clone(),
            }),
            GeneralTransaction::MpnWithdraw(mpn_withdraw) => {
                GeneralAddress::MpnAddress(MpnAddress {
                    pub_key: mpn_withdraw.zk_address.clone(),
                })
            }
        }
    }
}

impl From<TransactionAndDelta> for GeneralTransaction {
    fn from(tx: TransactionAndDelta) -> Self {
        Self::TransactionAndDelta(tx)
    }
}

impl From<MpnDeposit> for GeneralTransaction {
    fn from(tx: MpnDeposit) -> Self {
        Self::MpnDeposit(tx)
    }
}

impl From<MpnTransaction> for GeneralTransaction {
    fn from(tx: MpnTransaction) -> Self {
        Self::MpnTransaction(tx)
    }
}

impl From<MpnWithdraw> for GeneralTransaction {
    fn from(tx: MpnWithdraw) -> Self {
        Self::MpnWithdraw(tx)
    }
}

impl PartialEq<GeneralTransaction> for GeneralTransaction {
    fn eq(&self, other: &Self) -> bool {
        bincode::serialize(self).unwrap() == bincode::serialize(other).unwrap()
    }
}
impl Eq for GeneralTransaction {}
impl std::hash::Hash for GeneralTransaction {
    fn hash<Hasher>(&self, state: &mut Hasher)
    where
        Hasher: std::hash::Hasher,
    {
        state.write(&bincode::serialize(self).unwrap());
        state.finish();
    }
}
