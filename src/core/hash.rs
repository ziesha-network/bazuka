use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::fmt::Debug;

use super::{AutoDeserialize, AutoHash, AutoSerialize, MemberBound};

pub trait Hash: Debug + Clone + 'static {
    /// The length in bytes of the Hasher output
    const LENGTH: usize;

    type Output: MemberBound
        + AutoSerialize
        + AutoDeserialize
        + AutoHash
        + AsRef<[u8]>
        + AsMut<[u8]>
        + Default
        + Copy
        + PartialOrd;

    fn hash(s: &[u8]) -> Self::Output;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sha3Hasher;

impl Hash for Sha3Hasher {
    const LENGTH: usize = 32;
    // U32 is copy from the macro named impl_sha3 in RustCrypto
    type Output = [u8; 32];

    fn hash(s: &[u8]) -> Self::Output {
        let mut hasher = Sha3_256::new();
        hasher.update(s);
        hasher.finalize().into()
    }
}

#[cfg(test)]
mod tests {
    use crate::core::hash::Hash;
    use crate::core::hash::Sha3Hasher;

    #[test]
    fn test_sha3_works() {
        let _ = Sha3Hasher::hash(b"123");
    }
}
