mod address;
mod blocks;
pub mod hash;
mod header;
mod money;
mod transaction;

use crate::crypto;

pub use money::Money;

pub type Hasher = hash::Sha3Hasher;
pub type Signer = crypto::ed25519::Ed25519<Hasher>;

pub type ZkHasher = crate::zk::PoseidonHasher;
pub type ZkSigner = crypto::jubjub::JubJub<ZkHasher>;

pub type Address = address::Address<Signer>;
pub type ParseAddressError = address::ParseAddressError;
pub type Account = address::Account;
pub type Signature = address::Signature<Signer>;
pub type Transaction = transaction::Transaction<Hasher, Signer, ZkSigner>;
pub type TransactionData = transaction::TransactionData<Hasher, Signer, ZkSigner>;
pub type ContractAccount = transaction::ContractAccount;
pub type ContractUpdate = transaction::ContractUpdate<Hasher, Signer, ZkSigner>;
pub type ContractPayment = transaction::ContractPayment<Hasher, Signer, ZkSigner>;
pub type MpnPayment = transaction::MpnPayment<Hasher, Signer, ZkSigner>;
pub type PaymentDirection = transaction::PaymentDirection<Signer, ZkSigner>;
pub type Header = header::Header<Hasher>;
pub type Block = blocks::Block<Hasher, Signer, ZkSigner>;

pub type ProofOfWork = header::ProofOfWork;
pub type ContractId = transaction::ContractId<Hasher>;

pub type TransactionAndDelta = transaction::TransactionAndDelta<Hasher, Signer, ZkSigner>;
