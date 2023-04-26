use super::messages::{PostBlockRequest, PostBlockResponse};
use super::{promote_block, NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::db::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn post_block<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
    req: PostBlockRequest,
) -> Result<PostBlockResponse, NodeError> {
    let mut ctx = context.write().await;
    if req.block.header.number == ctx.blockchain.get_height()? {
        if req
            .block
            .header
            .proof_of_stake
            .timestamp
            .saturating_sub(ctx.network_timestamp())
            > ctx.opts.max_block_time_difference
        {
            return Err(NodeError::BlockTimestampInFuture);
        }
        ctx.blockchain
            .extend(req.block.header.number, &[req.block.clone()])?;
        ctx.on_update()?;
        drop(ctx);
        promote_block(context, req.block).await;
    }
    Ok(PostBlockResponse {})
}
