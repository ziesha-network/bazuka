#[cfg(feature = "node")]
use {
    bazuka::blockchain::KvStoreChain,
    bazuka::db::LevelDbKvStore,
    bazuka::node::{Node, NodeError},
    std::path::Path,
};

#[cfg(not(feature = "node"))]
use {
    bazuka::crypto::{Fr, MiMC},
    ff::Field,
};

#[cfg(feature = "node")]
#[macro_use]
extern crate lazy_static;

#[cfg(feature = "node")]
lazy_static! {
    static ref NODE: Node<KvStoreChain<LevelDbKvStore>> = Node::new(KvStoreChain::new(
        LevelDbKvStore::new(&home::home_dir().unwrap().join(Path::new(".bazuka")))
    ));
}

#[cfg(feature = "node")]
#[tokio::main]
async fn main() -> Result<(), NodeError> {
    NODE.run().await?;
    Ok(())
}

#[cfg(not(feature = "node"))]
fn main() {
    println!(
        "Genesis hash: {:?}",
        bazuka::config::genesis::get_genesis_block().hash()
    );

    let hasher = MiMC::new(b"mimc");
    println!(
        "MiMC output: {:?}",
        hasher.hash(&vec![Fr::zero(), Fr::one()])
    );
}
