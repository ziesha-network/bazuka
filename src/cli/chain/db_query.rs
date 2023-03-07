use crate::db::KvStore;
use crate::{cli::get_conf, db::ReadOnlyLevelDbKvStore};

pub fn db_query(prefix: String) {
    let conf = get_conf().unwrap();
    let rdb = ReadOnlyLevelDbKvStore::read_only(&conf.db, 64).unwrap();
    let db = rdb.snapshot();
    for (k, v) in db.pairs(prefix.into()).unwrap().into_iter() {
        println!("{} -> {}", k, v);
    }
}
