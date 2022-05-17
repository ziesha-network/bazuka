mod address;
mod blocks;
pub mod hash;
mod header;
mod transaction;

use crate::crypto;

pub type Money = u64;
pub type Signer = crypto::EdDSA;
pub type Hasher = hash::Sha3Hasher;
pub type Address = address::Address<Signer>;
pub type Account = address::Account;
pub type Signature = address::Signature<Signer>;
pub type Transaction = transaction::Transaction<Hasher, Signer>;
pub type TransactionData = transaction::TransactionData<Hasher, Signer>;
pub type Header = header::Header<Hasher>;
pub type Block = blocks::Block<Hasher, Signer>;

pub type ProofOfWork = header::ProofOfWork;
pub type ContractId = transaction::ContractId<Hasher>;
