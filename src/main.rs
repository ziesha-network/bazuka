#[macro_use]
extern crate lazy_static;

use bazuka::node::{Node, NodeError};

lazy_static! {
    static ref NODE: Node = Node::new();
}

#[tokio::main]
async fn main() -> Result<(), NodeError> {
    NODE.run().await?;
    Ok(())
}
