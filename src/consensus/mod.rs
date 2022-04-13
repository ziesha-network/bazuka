#[cfg(feature = "pos")]
pub mod pos;

#[cfg(feature = "pow")]
pub mod pow {
    use rust_randomx::{Context, Hasher, Output};
    use std::sync::Arc;

    lazy_static! {
        pub static ref HASHER: Hasher = Hasher::new(Arc::new(Context::new(b"bazuka", false)));
    }

    pub fn hash(_key: &[u8], input: &[u8]) -> Output {
        HASHER.hash(input)
    }
}
