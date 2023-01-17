use crate::crypto::{SignatureScheme, ZkSignatureScheme};
use std::str::FromStr;
use thiserror::Error;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MpnAddress<ZS: ZkSignatureScheme> {
    pub account_index: u64,
    pub pub_key: ZS::Pub,
}

#[derive(Error, Debug)]
pub enum ParseMpnAddressError {
    #[error("mpn address invalid")]
    Invalid,
}

impl<ZS: ZkSignatureScheme> std::fmt::Display for MpnAddress<ZS> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:x}{}", self.account_index, self.pub_key)?;
        Ok(())
    }
}

impl<ZS: ZkSignatureScheme> FromStr for MpnAddress<ZS> {
    type Err = ParseMpnAddressError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() <= 66 {
            return Err(ParseMpnAddressError::Invalid);
        }
        let account_index = u64::from_str_radix(&s[0..s.len() - 66], 16)
            .map_err(|_| ParseMpnAddressError::Invalid)?;
        if account_index > 0x3FFFFFFF {
            return Err(ParseMpnAddressError::Invalid);
        }
        let pub_key: ZS::Pub = s[s.len() - 66..]
            .parse()
            .map_err(|_| ParseMpnAddressError::Invalid)?;
        Ok(Self {
            account_index,
            pub_key,
        })
    }
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub enum Signature<S: SignatureScheme> {
    Unsigned,
    Signed(S::Sig),
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub struct Account {
    pub nonce: u32,
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub struct ZkAccount {
    pub nonce: u32,
}
