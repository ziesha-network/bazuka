use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

use crate::core::Hash;

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
    use crate::core::hash::Sha3Hasher;
    use crate::core::Hash;

    #[test]
    fn test_sha3_works() {
        let _ = Sha3Hasher::hash(b"123");
    }
}
