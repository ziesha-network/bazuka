use crate::core::{Amount, Ratio};
use crate::crypto::{SignatureScheme, VerifiableRandomFunction, ZkSignatureScheme};
use std::str::FromStr;
use thiserror::Error;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MpnAddress<ZS: ZkSignatureScheme> {
    pub pub_key: ZS::Pub,
}

#[derive(Error, Debug)]
pub enum ParseMpnAddressError {
    #[error("mpn address invalid")]
    Invalid,
}

impl<ZS: ZkSignatureScheme> std::fmt::Display for MpnAddress<ZS> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.pub_key)?;
        Ok(())
    }
}

impl<ZS: ZkSignatureScheme> FromStr for MpnAddress<ZS> {
    type Err = ParseMpnAddressError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            pub_key: s.parse().map_err(|_| ParseMpnAddressError::Invalid)?,
        })
    }
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum Signature<S: SignatureScheme> {
    Unsigned,
    Signed(S::Sig),
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Staker<V: VerifiableRandomFunction> {
    pub vrf_pub_key: V::Pub,
    pub commission: Ratio,
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Delegate {
    pub amount: Amount,
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Undelegation {
    pub amount: Amount,
    pub unlocks_on: u64, // Header number
}
