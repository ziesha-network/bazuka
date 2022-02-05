use super::primitives::{Address, Block, Money};

use db_key::Key;
use leveldb::database::Database;
use leveldb::kv::KV;
use leveldb::options::{Options, ReadOptions, WriteOptions};
use std::fs;
use std::path::Path;

trait Blockchain {
    fn get_balance(&self, addr: Address) -> Money;
    fn extend(&mut self, blocks: &Vec<Block>);
}

#[derive(Clone, Debug)]
struct MyKey(Vec<u8>);

impl Key for MyKey {
    fn from_u8(key: &[u8]) -> MyKey {
        MyKey(key.to_vec())
    }

    fn as_slice<T, F: Fn(&[u8]) -> T>(&self, f: F) -> T {
        f(&self.0)
    }
}

pub struct LevelDbChain {
    database: Database<MyKey>,
}

impl LevelDbChain {
    pub fn new(path: &Path) -> LevelDbChain {
        fs::create_dir_all(&path).unwrap();
        let mut options = Options::new();
        options.create_if_missing = true;
        LevelDbChain {
            database: Database::open(&path, options).unwrap(),
        }
    }
    pub fn check(&mut self) {
        let k = MyKey(vec![0u8, 1u8, 2u8]);

        let write_opts = WriteOptions::new();
        self.database.put(write_opts, k.clone(), &[1]).unwrap();

        let read_opts = ReadOptions::new();
        let res = self.database.get(read_opts, k.clone()).unwrap();
        println!("Data: {:?}", res);
    }
}
