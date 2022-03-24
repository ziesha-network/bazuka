mod curve;
mod eddsa;
mod field;
pub mod merkle;
mod mimc;
pub use eddsa::*;
pub use mimc::*;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::{Debug, Display};
use std::str::FromStr;

pub trait SignatureScheme: Clone + Serialize {
    type Pub: Clone + Debug + PartialEq + Serialize + DeserializeOwned + FromStr + Display;
    type Priv;
    type Sig: Clone + Debug + PartialEq + Serialize + DeserializeOwned;
    fn generate_keys(seed: &[u8]) -> (Self::Pub, Self::Priv);
    fn sign(sk: &Self::Priv, msg: &[u8]) -> Self::Sig;
    fn verify(pk: &Self::Pub, msg: &[u8], sig: &Self::Sig) -> bool;
}

pub trait VerifiableRandomFunction {
    type Pub;
    type Priv;
    type Output;
    type Proof;
    fn generate(seed: &[u8]) -> (Self::Pub, Self::Priv);
    fn evaluate(sk: &Self::Priv, input: &[u8]) -> (Self::Output, Self::Proof);
    fn verify(pk: &Self::Pub, input: &[u8], output: &Self::Output, proof: &Self::Proof) -> bool;
}
