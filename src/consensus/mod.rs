pub mod pow {
    use rust_randomx::{Context, Hasher, Output};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    lazy_static! {
        pub static ref HASHER: Arc<Mutex<HashMap<Vec<u8>, Hasher>>> =
            Arc::new(Mutex::new(HashMap::new()));
    }

    pub fn hash(key: &[u8], input: &[u8]) -> Output {
        // TODO: Should not keep all of hashers!
        let mut hasher = HASHER.lock().unwrap();
        let key = key.to_vec();
        hasher
            .entry(key.clone())
            .or_insert_with(|| Hasher::new(Arc::new(Context::new(&key, false))))
            .hash(input)
    }
}
