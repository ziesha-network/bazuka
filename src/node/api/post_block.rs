use super::messages::{PostBlockRequest, PostBlockResponse};
use super::{promote_block, NodeContext, NodeError};
use crate::blockchain::{BlockAndPatch, Blockchain};
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn post_block<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: PostBlockRequest,
) -> Result<PostBlockResponse, NodeError> {
    let mut ctx = context.write().await;
    if req.block.header.number == ctx.blockchain.get_height()? {
        ctx.blockchain
            .extend(req.block.header.number, &[req.block.clone()])?;
        ctx.on_update()?;
        ctx.blockchain.update_states(&req.patch)?;
        drop(ctx);
        promote_block(
            context,
            BlockAndPatch {
                block: req.block,
                patch: req.patch,
            },
        )
        .await;
    }
    Ok(PostBlockResponse {})
}
