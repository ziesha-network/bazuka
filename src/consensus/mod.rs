#[cfg(feature = "pow")]
pub mod pow {
    use rust_randomx::{Context, Hasher, Output};
    use std::sync::{Arc, Mutex};

    lazy_static! {
        pub static ref HASHER: Arc<Mutex<Option<Hasher>>> = Arc::new(Mutex::new(None));
    }

    pub fn hash(key: &[u8], input: &[u8]) -> Output {
        let mut hasher = HASHER.lock().unwrap();
        if hasher.is_none() || hasher.as_ref().unwrap().context().key() != key {
            log::info!("Initializing RandomX hasher...");
            *hasher = Some(Hasher::new(Arc::new(Context::new(key, false))));
        }
        hasher.as_ref().unwrap().hash(input)
    }
}
