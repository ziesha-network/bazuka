use std::fmt::Debug;
use std::str::FromStr;

use num_traits::{One, Zero};
use thiserror::Error;

use crate::core::number::U256;
use crate::crypto;
use crate::crypto::SignatureScheme;

pub mod blocks;
pub mod digest;
pub mod hash;
pub mod header;
pub mod number;

pub type BlockNumU64 = u64;
pub type Sha3_256 = crate::core::hash::Sha3Hasher;
pub type Header = crate::core::header::Header<Sha3_256, BlockNumU64>;
pub type Block = crate::core::blocks::Block<Header>;

macro_rules! auto_trait {
    (
        $(
            $(#[$doc:meta])+
            trait $name:ident: $( $bound:path ),+;
        )+
    ) => {
        $(
            $(#[$doc])+
            pub trait $name: $( $bound + )+ {}
            impl <T: $($bound +)+> $name for T {}
        )+
    };
}

auto_trait!(
    /// A type that implements Serialize in node runtime
    trait AutoSerialize: serde::ser::Serialize;

    /// A type that implements Deserialize in node runtime
    trait AutoDeserialize: serde::de::DeserializeOwned;
    /// A type that implements Hash in node runtime
    trait AutoHash: core::hash::Hash;
    /// A type that implements Display in runtime
    trait AutoDisplay: core::fmt::Display;
    /// A type that implements CanBe32Bits
    trait CanBe32Bits: core::convert::From<u32>;
);

/// A type that can be used at runtime
pub trait MemberBound: Send + Sync + Sized + Debug + Clone + Eq + PartialEq + 'static {}
impl<T: Send + Sync + Sized + Debug + Clone + Eq + PartialEq + 'static> MemberBound for T {}

pub trait Hash: Debug + Clone + 'static {
    /// The length in bytes of the Hasher output
    const LENGTH: usize;

    type Output: MemberBound
        + AutoSerialize
        + AutoDeserialize
        + AutoHash
        + AsRef<[u8]>
        + AsMut<[u8]>
        + Default
        + Copy;

    fn hash(s: &[u8]) -> Self::Output;
}

/// Number as a type in Header
pub trait BlockNumber: Default + Copy + Into<U256> + TryFrom<U256> + Eq + Zero + One {}
impl BlockNumber for BlockNumU64 {}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub enum Signature {
    Unsigned,
    Signed(crypto::Signature),
}

pub type Money = u64;

// All of the Zeeka's supply exists in Treasury account when the blockchain begins.
// Validator/Miner fees are collected from the Treasury account. This simplifies
// the process of money creation.
#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub enum Address {
    Treasury,
    PublicKey(crypto::PublicKey),
}

#[derive(Error, Debug)]
pub enum ParseAddressError {
    #[error("address invalid")]
    Invalid,
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Address::Treasury => write!(f, "Treasury"),
            Address::PublicKey(pk) => write!(f, "{}", pk),
        }
    }
}

impl FromStr for Address {
    type Err = ParseAddressError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Address::PublicKey(crypto::PublicKey::from_str(s).unwrap()))
    }
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub struct ContractId {}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub struct HashOutput {}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub struct EntryExit {
    src: Address,
    amount: Money,
    sig: Signature,
    fee: Money,
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub struct ZkProof {
    proof: u8,
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub struct ZkVerifyingKey {}

// A transaction could be as simple as sending some funds, or as complicated as
// creating a smart-contract.
#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub enum TransactionData {
    RegularSend {
        dst: Address,
        amount: Money,
    },
    RegisterValidator {
        vrf_stuff: u8,
        amount: Money,
    },

    // Create a Zero-Contract. The creator can consider multiple ways (Circuits) of updating
    // the state. But there should be only one circuit for entering and exiting the contract.
    CreateContract {
        entry_circuit: ZkVerifyingKey,
        update_circuits: Vec<ZkVerifyingKey>,
        exit_circuit: ZkVerifyingKey,
        initial_state: HashOutput,
    },
    // Proof for EntryCircuit(curr_state, next_state, hash(entries))
    ProcessEntries {
        contract_id: ContractId,
        entries: Vec<EntryExit>,
        next_state: HashOutput,
        proof: ZkProof,
    },
    // Proof for UpdateCircuit[circuit_index](curr_state, next_state)
    Update {
        contract_id: ContractId,
        circuit_index: u32,
        next_state: HashOutput,
        proof: ZkProof,
    },
    // Proof for ExitCircuit(curr_state, next_state, hash(entries))
    ProcessExits {
        contract_id: ContractId,
        exits: Vec<EntryExit>,
        next_state: HashOutput,
        proof: ZkProof,
    },
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub struct Transaction {
    pub src: Address,
    pub nonce: u32,
    pub data: TransactionData,
    pub fee: Money,
    pub sig: Signature,
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub struct Account {
    pub balance: Money,
    pub nonce: u32,
}

impl Transaction {
    pub fn verify_signature(&self) -> bool {
        match &self.src {
            Address::Treasury => true,
            Address::PublicKey(pk) => match &self.sig {
                Signature::Unsigned => false,
                Signature::Signed(sig) => {
                    let mut unsigned = self.clone();
                    unsigned.sig = Signature::Unsigned;
                    let bytes = bincode::serialize(&unsigned).unwrap();
                    crypto::EdDSA::verify(&pk, &bytes, &sig)
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {
        assert_eq!(1, 1)
    }
}
