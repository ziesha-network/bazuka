use std::fmt::{Debug, Display};
use std::str::FromStr;

use crate::zk;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub mod merkle;

pub mod ed25519;
pub mod jubjub;

pub trait SignatureScheme: Clone + Serialize {
    type PubParseError;
    type Pub: Clone
        + Debug
        + PartialEq
        + Eq
        + std::hash::Hash
        + Serialize
        + DeserializeOwned
        + FromStr
        + Display
        + From<Self::Priv>
        + Default; // Default is null pub-key (Treasury address)
    type Priv: Clone;
    type Sig: Clone + Debug + PartialEq + Serialize + DeserializeOwned;
    fn generate_keys(seed: &[u8]) -> (Self::Pub, Self::Priv);
    fn sign(sk: &Self::Priv, msg: &[u8]) -> Self::Sig;
    fn verify(pk: &Self::Pub, msg: &[u8], sig: &Self::Sig) -> bool;
}

pub trait ZkSignatureScheme: Clone + Serialize {
    type Pub: Clone
        + Debug
        + PartialEq
        + Eq
        + std::hash::Hash
        + Serialize
        + DeserializeOwned
        + FromStr
        + Display;
    type Priv: Clone;
    type Sig: Clone + Debug + PartialEq + Serialize + DeserializeOwned;
    fn generate_keys(seed: &[u8]) -> (Self::Pub, Self::Priv);
    fn sign(sk: &Self::Priv, msg: zk::ZkScalar) -> Self::Sig;
    fn verify(pk: &Self::Pub, msg: zk::ZkScalar, sig: &Self::Sig) -> bool;
}
