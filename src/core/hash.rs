use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::fmt::Debug;

pub trait Hash: Debug + Clone + Serialize + 'static {
    type Output: Debug
        + Serialize
        + DeserializeOwned
        + AsRef<[u8]>
        + AsMut<[u8]>
        + Default
        + Copy
        + PartialOrd
        + PartialEq
        + Eq
        + TryFrom<Vec<u8>>;

    fn hash(s: &[u8]) -> Self::Output;
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Copy, Eq, std::hash::Hash)]
pub struct Sha3Hasher;

impl Hash for Sha3Hasher {
    type Output = [u8; 32];

    fn hash(s: &[u8]) -> Self::Output {
        let mut h = Sha3_256::new();
        h.update(s);
        h.finalize().into()
    }
}
