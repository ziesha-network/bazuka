use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use sha3::digest::core_api::CoreWrapper;
use sha3::{Digest, Sha3_256, Sha3_256Core};

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
        + Copy;

    fn hash(s: &[u8]) -> Self::Output;

    fn update(&mut self, s: &[u8]);

    fn finalize(self) -> Self::Output;
}

#[derive(Debug, Clone)]
pub struct Sha3Hasher(Option<CoreWrapper<Sha3_256Core>>);

impl Default for Sha3Hasher {
    fn default() -> Self {
        Sha3Hasher(None)
    }
}

impl Sha3Hasher {
    pub fn new() -> Self {
        Self(Some(Sha3_256::new()))
    }
}

impl Hash for Sha3Hasher {
    const LENGTH: usize = 32;
    // U32 is copy from the macro named impl_sha3 in RustCrypto
    type Output = [u8; 32];

    fn hash(s: &[u8]) -> Self::Output {
        let mut h = Sha3_256::new();
        h.update(s);
        h.finalize().into()
    }

    fn update(&mut self, s: &[u8]) {
        if self.0.is_none() {
            self.0 = Some(Sha3_256::new())
        }
        self.0.as_mut().map(|mut h| {
            h.update(s);
        });
    }

    fn finalize(self) -> Self::Output {
        assert!(self.0.is_some());
        // self.0.as_ref().map(|h| (*h).finalize().into()).unwrap()
        self.0.map(|h| h.finalize().into()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::core::hash::Hash;
    use crate::core::hash::Sha3Hasher;

    #[test]
    fn test_sha3_works() {
        let output1 = Sha3Hasher::hash(b"123");

        let mut sha3 = Sha3Hasher::default();
        sha3.update(b"123");
        let output2 = sha3.finalize();
        assert_eq!(output1, output2)
    }
}
