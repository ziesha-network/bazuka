use crate::blockchain::Blockchain;
use crate::{
    blockchain::KvStoreChain,
    cli::{get_conf},
    config::blockchain::get_blockchain_config,
    db::LevelDbKvStore,
};

pub async fn rollback() {
    let conf = get_conf().unwrap();
    let mut chain = KvStoreChain::new(
        LevelDbKvStore::new(&conf.db, 64).unwrap(),
        get_blockchain_config(),
    )
    .unwrap();
    chain.rollback().unwrap();
}
