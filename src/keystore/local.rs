use std::collections::HashMap;
use std::path::{Path, PathBuf};

use secrecy::{ExposeSecret, SecretString};

use crate::core::crypto::KeyId;
use crate::keystore::{Error, Result};

struct KeyStoreInner {
    path: Option<PathBuf>,
    // key_id, raw public key, Key phrase/seed
    key_pairs: HashMap<(KeyId, Vec<u8>), String>,
    password: Option<SecretString>,
}

impl KeyStoreInner {
    fn open<T: Into<PathBuf>>(path: T, password: Option<SecretString>) -> Result<Self> {
        let path = path.into();
        std::fs::create_dir_all(&path)?;

        Ok(Self {
            path: Some(path),
            key_pairs: HashMap::new(),
            password,
        })
    }

    fn new_in_memory() -> Self {
        Self {
            path: None,
            key_pairs: Default::default(),
            password: None,
        }
    }

    fn password(&self) -> Option<&str> {
        self.password
            .as_ref()
            .map(|p| p.expose_secret())
            .map(|p| p.as_ref())
    }

    fn get_seed(&self, key_id: KeyId, public: &[u8]) -> Option<&String> {
        let key = (key_id, public.to_vec());
        self.key_pairs.get(&key)
    }
}
