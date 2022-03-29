use std::fmt::{Debug, Display};
use std::str::FromStr;

use schnorrkel::vrf::VRFProof;
use serde::de::DeserializeOwned;
use serde::Serialize;

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

pub trait VerifiableRandomFunction: Sized {
    type Pub;
    type Output: AsRef<[u8]>;
    type Proof: AsRef<[u8]>;

    fn generate(seed: &[u8]) -> Result<Self, Error>;

    fn sign(&self, transcript: VRFTranscript) -> (Self::Output, Self::Proof);

    fn verify(
        public_key: &Self::Pub,
        transcript: VRFTranscript,
        output: Self::Output,
        proof: Self::Proof,
    ) -> Result<Vec<u8>, Error>;
}

pub trait PublicKey: AsRef<[u8]> + Display {}

impl<T: AsRef<[u8]> + Display> PublicKey for T {}

#[derive(Clone)]
pub enum VRFTranscriptData {
    U64(u64),
    Bytes(Vec<u8>),
}

#[derive(Clone)]
pub struct VRFTranscript {
    pub label: &'static [u8],
    pub messages: Vec<(&'static [u8], VRFTranscriptData)>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("vrf error: {0}")]
    VRFSignatureError(String),
    #[error("the {0} has an invalid length")]
    InvalidLength(String),
    #[error("the seed is invalid")]
    InvalidSeed,
}
