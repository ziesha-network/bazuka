#[macro_use]
extern crate lazy_static;

use bazuka::blockchain::check_db;
use bazuka::node::{Node, NodeError};

lazy_static! {
    static ref NODE: Node = Node::new();
}

#[tokio::main]
async fn main() -> Result<(), NodeError> {
    check_db();
    NODE.run().await?;
    Ok(())
}
