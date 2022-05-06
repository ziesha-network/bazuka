use super::messages::{GetBlocksRequest, GetBlocksResponse};
use super::{Network, NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::config::MAX_BLOCK_FETCH;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_blocks<B: Blockchain, N: Network>(
    context: Arc<RwLock<NodeContext<N, B>>>,
    req: GetBlocksRequest,
) -> Result<GetBlocksResponse, NodeError> {
    let context = context.read().await;
    let height = context.blockchain.get_height()?;
    let until = std::cmp::min(height, req.since + MAX_BLOCK_FETCH);
    Ok(GetBlocksResponse {
        blocks: context.blockchain.get_blocks(req.since, Some(until))?,
    })
}
