use std::fmt::{Debug, Display};
use std::str::FromStr;

use serde::de::DeserializeOwned;
use serde::Serialize;

pub use eddsa::*;
pub use eddsa::*;
pub use mimc::*;
pub use vrf::*;

mod curve;
mod eddsa;
mod field;
pub mod merkle;
mod mimc;
mod vrf;

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
    fn generate() -> Self;
    fn evaluate(&self, input: &[u8]) -> (Self::Output, Self::Proof);
    fn verify(pk: &Self::Pub, input: &[u8], output: &Self::Output, proof: &Self::Proof) -> bool;
}

pub trait PublicKey: AsRef<[u8]> + Display {}

impl<T: AsRef<[u8]> + Display> PublicKey for T {}
