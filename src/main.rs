use bazuka::node::{Node, NodeError};

#[tokio::main]
async fn main() -> Result<(), NodeError> {
    let n = Node::new();
    n.run().await?;
    Ok(())
}
