use crate::cli::BazukaConfig;
use bazuka::db::KvStore;
use bazuka::db::ReadOnlyLevelDbKvStore;

pub fn db_query(prefix: String, conf: &BazukaConfig) {
    let rdb = ReadOnlyLevelDbKvStore::read_only(&conf.db, 64).unwrap();
    let db = rdb.snapshot();
    for (k, v) in db.pairs(prefix.into()).unwrap().into_iter() {
        println!("{} -> {}", k, v);
    }
}
