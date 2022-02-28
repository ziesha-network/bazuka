#[cfg(feature = "node")]
use {
    bazuka::blockchain::KvStoreChain,
    bazuka::db::LevelDbKvStore,
    bazuka::node::{Node, NodeError},
    std::path::Path,
};

#[cfg(not(feature = "node"))]
use {bazuka::core::Address, bazuka::wallet::Wallet};

#[cfg(feature = "node")]
#[macro_use]
extern crate lazy_static;

#[cfg(feature = "node")]
lazy_static! {
    static ref NODE: Node<KvStoreChain<LevelDbKvStore>> = Node::new(
        "http://127.0.0.1:3030".to_string(),
        KvStoreChain::new(LevelDbKvStore::new(
            &home::home_dir().unwrap().join(Path::new(".bazuka"))
        ))
        .unwrap()
    );
}

#[cfg(feature = "node")]
#[tokio::main]
async fn main() -> Result<(), NodeError> {
    NODE.run().await?;
    Ok(())
}

#[cfg(not(feature = "node"))]
fn main() {
    println!("Bazuka!");
    let wallet = Wallet::new(b"random seed".to_vec());
    println!("Your address is: {:?}", wallet.get_address());
    let tx = wallet.create_transaction(Address::Nowhere, 123);
    println!("Verify tx signature: {}", tx.verify_signature());
}
