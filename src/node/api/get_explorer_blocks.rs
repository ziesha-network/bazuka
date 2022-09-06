use super::messages::{GetExplorerBlocksRequest, GetExplorerBlocksResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_explorer_blocks<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: GetExplorerBlocksRequest,
) -> Result<GetExplorerBlocksResponse, NodeError> {
    let context = context.read().await;
    let count = std::cmp::min(context.opts.max_blocks_fetch, req.count);
    let blocks = context.blockchain.get_blocks(req.since, count)?;
    Ok(GetExplorerBlocksResponse {
        pow_hashes: vec![], // TODO: Also provide PoW hashes
        blocks: blocks.iter().map(|b| b.into()).collect(),
    })
}
