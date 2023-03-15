use crate::core::Amount;
use crate::crypto::{
    DeriveMpnAccountIndex, SignatureScheme, VerifiableRandomFunction, ZkSignatureScheme,
};
use std::str::FromStr;
use thiserror::Error;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MpnAddress<ZS: ZkSignatureScheme>
where
    ZS::Pub: DeriveMpnAccountIndex,
{
    pub pub_key: ZS::Pub,
}

impl<ZS: ZkSignatureScheme> MpnAddress<ZS>
where
    ZS::Pub: DeriveMpnAccountIndex,
{
    pub fn account_index(&self, mpn_log4_account_capacity: u8) -> u64 {
        self.pub_key.mpn_account_index(mpn_log4_account_capacity)
    }
}

#[derive(Error, Debug)]
pub enum ParseMpnAddressError {
    #[error("mpn address invalid")]
    Invalid,
}

impl<ZS: ZkSignatureScheme> std::fmt::Display for MpnAddress<ZS>
where
    ZS::Pub: DeriveMpnAccountIndex,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.pub_key)?;
        Ok(())
    }
}

impl<ZS: ZkSignatureScheme> FromStr for MpnAddress<ZS>
where
    ZS::Pub: DeriveMpnAccountIndex,
{
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
pub struct Account {
    pub nonce: u32,
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Staker<V: VerifiableRandomFunction> {
    pub vrf_pub_key: V::Pub,
    pub commision: u8,
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Delegate {
    pub amount: Amount,
}
