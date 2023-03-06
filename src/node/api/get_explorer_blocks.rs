use super::messages::{GetExplorerBlocksRequest, GetExplorerBlocksResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::db::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_explorer_blocks<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
    req: GetExplorerBlocksRequest,
) -> Result<GetExplorerBlocksResponse, NodeError> {
    let context = context.read().await;
    let count = std::cmp::min(context.opts.max_blocks_fetch, req.count);
    let blocks = context.blockchain.get_blocks(req.since, count)?;
    Ok(GetExplorerBlocksResponse {
        blocks: blocks.iter().map(|b| b.into()).collect(),
    })
}
