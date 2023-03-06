use super::messages::{GetBlocksRequest, GetBlocksResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::db::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_blocks<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
    req: GetBlocksRequest,
) -> Result<GetBlocksResponse, NodeError> {
    let context = context.read().await;
    let count = std::cmp::min(context.opts.max_blocks_fetch, req.count);
    Ok(GetBlocksResponse {
        blocks: context.blockchain.get_blocks(req.since, count)?,
    })
}
