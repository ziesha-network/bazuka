#[cfg(feature = "pos")]
pub mod pos;

#[cfg(feature = "pow")]
pub mod pow {
    use rust_randomx::{Context, Hasher};
    use std::sync::Arc;

    lazy_static! {
        pub static ref HASHER: Hasher = Hasher::new(Arc::new(Context::new(b"bazuka", false)));
    }

    pub fn leading_zeros(_key: &[u8], input: &[u8]) -> u32 {
        HASHER.hash(input).leading_zeros()
    }
}
