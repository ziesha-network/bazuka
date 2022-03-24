mod address;
mod blocks;
mod contract;
mod header;
mod transaction;

pub mod digest;
pub mod hash;
pub mod number;

use std::fmt::Debug;

use crate::crypto;

pub type Money = u64;
pub type Sha3_256 = hash::Sha3Hasher;
pub type Address = address::Address<crypto::EdDSA>;
pub type Signature = address::Signature<crypto::EdDSA>;
pub type Transaction = transaction::Transaction<crypto::EdDSA>;
pub type TransactionData = transaction::TransactionData<crypto::EdDSA>;
pub type Header = header::Header<Sha3_256>;
pub type Block = blocks::Block<Sha3_256, crypto::EdDSA>;

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

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub struct Account {
    pub balance: Money,
    pub nonce: u32,
}

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {
        assert_eq!(1, 1)
    }
}
