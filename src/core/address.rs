use crate::crypto::SignatureScheme;
use std::str::FromStr;
use thiserror::Error;

// All of the Zeeka's supply exists in Treasury account when the blockchain begins.
// Validator/Miner fees are collected from the Treasury account. This simplifies
// the process of money creation.
#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub enum Address<S: SignatureScheme> {
    Treasury,
    PublicKey(S::Pub),
}

#[derive(Error, Debug)]
pub enum ParseAddressError {
    #[error("address invalid")]
    Invalid,
}

impl<S: SignatureScheme> std::fmt::Display for Address<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Address::<S>::Treasury => write!(f, "Treasury"),
            Address::<S>::PublicKey(pk) => write!(f, "{}", pk),
        }
    }
}

impl<S: SignatureScheme> FromStr for Address<S>
where
    <S::Pub as FromStr>::Err: std::fmt::Debug,
{
    type Err = ParseAddressError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Address::<S>::PublicKey(S::Pub::from_str(s).unwrap()))
    }
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub enum Signature<S: SignatureScheme> {
    Unsigned,
    Signed(S::Sig),
}
