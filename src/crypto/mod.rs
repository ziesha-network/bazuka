use std::fmt::{Debug, Display};
use std::str::FromStr;

use serde::de::DeserializeOwned;
use serde::Serialize;

pub use eddsa::*;

mod eddsa;
pub mod merkle;

pub trait SignatureScheme: Clone + Serialize {
    type Pub: Clone + Debug + PartialEq + Serialize + DeserializeOwned + FromStr + Display;
    type Priv;
    type Sig: Clone + Debug + PartialEq + Serialize + DeserializeOwned;
    fn generate_keys(seed: &[u8]) -> (Self::Pub, Self::Priv);
    fn sign(sk: &Self::Priv, msg: &[u8]) -> Self::Sig;
    fn verify(pk: &Self::Pub, msg: &[u8], sig: &Self::Sig) -> bool;
}
