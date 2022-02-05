#[macro_use]
extern crate lazy_static;

use bazuka::blockchain::LevelDbChain;
use bazuka::node::{Node, NodeError};
use std::path::Path;

lazy_static! {
    static ref NODE: Node = Node::new();
}

#[tokio::main]
async fn main() -> Result<(), NodeError> {
    let path = home::home_dir().unwrap().join(Path::new(".bazuka"));
    let mut chain = LevelDbChain::new(&path);
    chain.check();

    NODE.run().await?;
    Ok(())
}
