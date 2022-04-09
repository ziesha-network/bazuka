mod address;
mod blocks;
mod contract;
mod header;
mod transaction;

#[cfg(feature = "pos")]
pub mod digest;

pub mod hash;
pub mod number;

use std::fmt::Debug;

use crate::crypto;

pub type Money = u64;
pub type Signer = crypto::EdDSA;
pub type Hasher = hash::Sha3Hasher;
pub type Address = address::Address<Signer>;
pub type Account = address::Account;
pub type Signature = address::Signature<Signer>;
pub type Transaction = transaction::Transaction<Signer>;
pub type TransactionData = transaction::TransactionData<Signer>;
pub type Header = header::Header<Hasher>;
pub type Block = blocks::Block<Hasher, Signer>;

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
