use super::messages::{GenerateBlockRequest, GenerateBlockResponse};
use super::{promote_block, NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn generate_block<B: Blockchain>(
    client: Option<SocketAddr>,
    context: Arc<RwLock<NodeContext<B>>>,
    _req: GenerateBlockRequest,
) -> Result<GenerateBlockResponse, NodeError> {
    let mut ctx = context.write().await;
    let is_local = client.map(|c| c.ip().is_loopback()).unwrap_or(false);
    //if is_local {
    let wallet = ctx.wallet.clone();
    // Invoke PoS block generation
    if let Some(draft) = ctx.try_produce(wallet)? {
        drop(ctx);
        promote_block(context, draft).await;
        return Ok(GenerateBlockResponse { success: true });
    }
    //}
    Ok(GenerateBlockResponse { success: false })
}
