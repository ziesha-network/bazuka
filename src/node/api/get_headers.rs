use super::messages::{GetHeadersRequest, GetHeadersResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::db::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_headers<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
    req: GetHeadersRequest,
) -> Result<GetHeadersResponse, NodeError> {
    let context = context.read().await;
    let count = std::cmp::min(context.opts.max_blocks_fetch, req.count);
    let headers = context.blockchain.get_headers(req.since, count)?;
    Ok(GetHeadersResponse { headers })
}
