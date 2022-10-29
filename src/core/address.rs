use super::Money;
use crate::crypto::SignatureScheme;
use std::str::FromStr;
use thiserror::Error;

// All of the Zeeka's supply exists in Treasury account when the blockchain begins.
// Validator/Miner fees are collected from the Treasury account. This simplifies
// the process of money creation.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum Address<S: SignatureScheme> {
    Treasury,
    PublicKey(S::Pub),
}

impl<S: SignatureScheme> PartialEq for Address<S> {
    fn eq(&self, other: &Self) -> bool {
        bincode::serialize(self).unwrap() == bincode::serialize(other).unwrap()
    }
}

impl<S: SignatureScheme> Eq for Address<S> {}

impl<S: SignatureScheme> std::hash::Hash for Address<S> {
    fn hash<Hasher>(&self, state: &mut Hasher)
    where
        Hasher: std::hash::Hasher,
    {
        state.write(&bincode::serialize(self).unwrap());
        state.finish();
    }
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
        Ok(Address::<S>::PublicKey(
            S::Pub::from_str(s).map_err(|_| ParseAddressError::Invalid)?,
        ))
    }
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub enum Signature<S: SignatureScheme> {
    Unsigned,
    Signed(S::Sig),
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub struct Account {
    pub balance: Money,
    pub nonce: u32,
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub struct ZkAccount {
    pub nonce: u32,
}
