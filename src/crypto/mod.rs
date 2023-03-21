use std::fmt::{Debug, Display};
use std::str::FromStr;

use crate::zk;
use rand::{CryptoRng, RngCore};
use serde::de::DeserializeOwned;
use serde::Serialize;

pub mod merkle;

pub mod ed25519;
pub mod jubjub;
pub mod vrf;

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
    type Sig: Clone + Debug + PartialEq + Eq + Serialize + DeserializeOwned + Display;
    fn generate_keys(seed: &[u8]) -> (Self::Pub, Self::Priv);
    fn sign(sk: &Self::Priv, msg: &[u8]) -> Self::Sig;
    fn verify(pk: &Self::Pub, msg: &[u8], sig: &Self::Sig) -> bool;
}

pub trait DeriveMpnAccountIndex {
    fn mpn_account_index(&self, log4_account_capacity: u8) -> u64;
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
        + Display
        + Into<Vec<zk::ZkScalar>>;
    type Priv: Clone;
    type Sig: Clone
        + Debug
        + PartialEq
        + Eq
        + Serialize
        + DeserializeOwned
        + Into<Vec<zk::ZkScalar>>;
    fn generate_keys(seed: &[u8]) -> (Self::Pub, Self::Priv);
    fn sign(sk: &Self::Priv, msg: zk::ZkScalar) -> Self::Sig;
    fn verify(pk: &Self::Pub, msg: zk::ZkScalar, sig: &Self::Sig) -> bool;
}

pub trait VerifiableRandomFunction: Clone + Serialize {
    type Pub: Clone
        + Debug
        + PartialEq
        + Eq
        + std::hash::Hash
        + Serialize
        + FromStr
        + DeserializeOwned
        + AsRef<[u8]>
        + Display;
    type Priv;
    type Out: Clone
        + Debug
        + PartialEq
        + Eq
        + std::hash::Hash
        + Serialize
        + DeserializeOwned
        + Into<f32>;
    type Proof: Clone + Debug + PartialEq + Eq + std::hash::Hash + Serialize + DeserializeOwned;
    fn generate_keys<R: RngCore + CryptoRng>(csprng: R) -> (Self::Pub, Self::Priv);
    fn sign(sk: &Self::Priv, msg: &[u8]) -> (Self::Out, Self::Proof);
    fn verify(pk: &Self::Pub, msg: &[u8], output: &Self::Out, proof: &Self::Proof) -> bool;
}
