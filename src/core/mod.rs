use std::convert::TryInto;
use std::fmt::{Debug, Formatter};

use rand::RngCore;
use serde::ser::Serialize;
use sha3::{Digest, Sha3_256};

use crate::core::digest::Digests;
use crate::core::number::U256;

pub mod blocks;
pub mod digest;
pub mod hash;
pub mod header;
pub mod number;

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

#[macro_export]
auto_trait!(
    /// A type that implements Serialize in node runtime
    trait AutoSerialize: serde::ser::Serialize;

    /// A type that implements Deserialize in node runtime
    trait AutoDeserialize: serde::de::DeserializeOwned, serde::ser::Serialize;
    /// A type that implements Hash in node runtime
    trait AutoHash: core::hash::Hash;
    /// A type that implements Display in runtime
    trait AutoDisplay: core::fmt::Display;
    /// A type that implements CanBe32Bits
    trait CanBe32Bits: core::convert::From<u32>;
);

/// A type that can be used at runtime
pub trait Field: Send + Sync + Sized + Debug + Clone + Eq + PartialEq + 'static {}
impl<T: Send + Sync + Sized + Debug + Clone + Eq + PartialEq + 'static> Field for T {}

pub trait Hash: Debug + Clone {
    /// The length in bytes of the Hasher output
    const LENGTH: usize;

    type Output: Field
        + AutoSerialize
        + AutoDeserialize
        + AutoHash
        + AsRef<[u8]>
        + AsMut<[u8]>
        + Default
        + Copy;

    fn hash(s: &[u8]) -> Self::Output;
}

pub type Signature = u8;
pub type Money = u32;

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
pub enum Address {
    Nowhere,
    PublicKey(u8),
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
pub enum Transaction {
    RegularSend {
        src: Address,
        dst: Address,
        amount: Money,
        sig: Signature,
    },
}

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {
        assert_eq!(1, 1)
    }
}
