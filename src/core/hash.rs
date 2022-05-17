use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::fmt::Debug;

pub trait Hash: Debug + Clone + Serialize + 'static {
    /// The length in bytes of the Hasher output
    const LENGTH: usize;

    type Output: Debug
        + Serialize
        + DeserializeOwned
        + AsRef<[u8]>
        + AsMut<[u8]>
        + Default
        + Copy
        + PartialOrd;

    fn hash(s: &[u8]) -> Self::Output;
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Sha3Hasher;

impl Hash for Sha3Hasher {
    const LENGTH: usize = 32;
    // U32 is copy from the macro named impl_sha3 in RustCrypto
    type Output = [u8; 32];

    fn hash(s: &[u8]) -> Self::Output {
        let mut h = Sha3_256::new();
        h.update(s);
        h.finalize().into()
    }
}
