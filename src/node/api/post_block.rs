use super::messages::{PostBlockRequest, PostBlockResponse};
use super::{Network, NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn post_block<B: Blockchain, N: Network>(
    context: Arc<RwLock<NodeContext<N, B>>>,
    req: PostBlockRequest,
) -> Result<PostBlockResponse, NodeError> {
    let mut context = context.write().await;
    context
        .blockchain
        .extend(req.block.header.number as usize, &[req.block])?;
    Ok(PostBlockResponse {})
}
