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
    type Sig: Clone + Debug + PartialEq + Eq + Serialize + DeserializeOwned;
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
        + Display;
    type Priv: Clone;
    type Sig: Clone + Debug + PartialEq + Eq + Serialize + DeserializeOwned;
    fn generate_keys(seed: &[u8]) -> (Self::Pub, Self::Priv);
    fn sign(sk: &Self::Priv, msg: zk::ZkScalar) -> Self::Sig;
    fn verify(pk: &Self::Pub, msg: zk::ZkScalar, sig: &Self::Sig) -> bool;
}

pub trait VerifiableRandomFunction: Clone + Serialize {
    type Ctx;
    type Pub: Clone
        + Debug
        + PartialEq
        + Eq
        + std::hash::Hash
        + Serialize
        + FromStr
        + DeserializeOwned
        + AsRef<[u8]>;
    type Priv;
    type Sig;
    fn make_context(bytes: &[u8]) -> Self::Ctx;
    fn generate_keys<R: RngCore + CryptoRng>(csprng: R) -> (Self::Pub, Self::Priv);
    fn sign(sk: &Self::Priv, ctx: &Self::Ctx, msg: &[u8]) -> Self::Sig;
    fn verify(pk: &Self::Pub, ctx: &Self::Ctx, msg: &[u8], sig: &Self::Sig) -> bool;
}
