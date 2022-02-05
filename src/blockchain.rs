use super::primitives::{Address, Block, Money};

use leveldb::database::Database;
use leveldb::kv::KV;
use leveldb::options::{Options, ReadOptions, WriteOptions};
use std::fs;
use std::path::Path;

trait Blockchain {
    fn get_balance(&self, addr: Address) -> Money;
    fn extend(&mut self, blocks: &Vec<Block>);
}

pub fn check_db() {
    let path = home::home_dir().unwrap().join(Path::new(".bazuka"));
    fs::create_dir_all(&path).unwrap();
    let mut options = Options::new();
    options.create_if_missing = true;
    let database = Database::open(&path, options).unwrap();

    let write_opts = WriteOptions::new();
    database.put(write_opts, 1, &[1]).unwrap();

    let read_opts = ReadOptions::new();
    let res = database.get(read_opts, 1).unwrap();
    println!("Data: {:?}", res);
}
