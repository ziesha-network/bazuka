pub mod pow {
    use rust_randomx::{Context, Hasher};
    use serde::{Deserialize, Serialize};
    use std::cmp::Ordering;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
    pub struct Difficulty(pub u32);

    impl Difficulty {
        pub fn power(&self) -> u128 {
            rust_randomx::Difficulty::new(self.0).power()
        }
    }

    impl Ord for Difficulty {
        fn cmp(&self, other: &Self) -> Ordering {
            self.power().cmp(&other.power())
        }
    }

    impl PartialOrd for Difficulty {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    lazy_static! {
        pub static ref HASHER: Arc<Mutex<HashMap<Vec<u8>, Hasher>>> =
            Arc::new(Mutex::new(HashMap::new()));
    }

    pub fn meets_difficulty(key: &[u8], input: &[u8], diff: Difficulty) -> bool {
        let mut hasher = HASHER.lock().unwrap();

        #[cfg(not(test))]
        hasher.retain(|k, _| k == &key);

        let key = key.to_vec();
        hasher
            .entry(key.clone())
            .or_insert_with(|| Hasher::new(Arc::new(Context::new(&key, false))))
            .hash(input)
            .meets_difficulty(rust_randomx::Difficulty::new(diff.0))
    }
}
