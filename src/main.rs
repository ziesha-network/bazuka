#[macro_use]
extern crate lazy_static;

use bazuka::blockchain::KvStoreChain;
use bazuka::crypto::{Fr, MiMC};
use bazuka::db::LevelDbKvStore;
use bazuka::node::{Node, NodeError};
use ff::Field;
use std::path::Path;

lazy_static! {
    static ref NODE: Node<KvStoreChain<LevelDbKvStore>> = Node::new(KvStoreChain::new(
        LevelDbKvStore::new(&home::home_dir().unwrap().join(Path::new(".bazuka")))
    ));
}

#[tokio::main]
async fn main() -> Result<(), NodeError> {
    println!(
        "Genesis hash: {:?}",
        bazuka::genesis::get_genesis_block().hash()
    );

    let hasher = MiMC::new(b"mimc");
    println!(
        "MiMC output: {:?}",
        hasher.hash(&vec![Fr::zero(), Fr::one()])
    );

    NODE.run().await?;
    Ok(())
}
