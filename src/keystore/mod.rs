use schnorrkel::vrf::{VRFOutput, VRFPreOut, VRFProof};
use std::fmt::{Display, Formatter};

mod local;

#[derive(Debug, derive_more::Display, derive_more::From)]
pub enum Error {
    IoError(std::io::Error),
    #[display(fmt = "Invalid password")]
    InvalidPassword,
    #[display(fmt = "Invalid recovery phrase data")]
    InvalidPhrase,
    #[display(fmt = "Invalid seed")]
    InvalidSeed,
    #[display(fmt = "Key crypto type is not supported")]
    KeyNotSupported(KeyTypeId),
    #[display(fmt = "Keystore unavailable")]
    Unavailable,
}

pub type Result<T> = std::result::Result<T, Error>;

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::IoError(ref error) => Some(error),
            _ => None,
        }
    }
}

pub struct VRFSignature {
    pub output: VRFPreOut,
    pub proof: VRFProof,
}

#[async_trait::async_trait]
pub trait CryptoApi: Sync + Send {
    async fn sr25519_public_keys(&self);
    async fn vrf_sign(&self) -> Result<VRFSignature>;
    fn has_keys(&self, public_key: &[u8]) -> bool;
}

pub trait SyncCryptoApi: CryptoApi {
    fn sr25519_public_keys(&self);
    fn vrf_sign(&self) -> Result<VRFSignature>;
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
