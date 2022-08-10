use super::messages::{GetHeadersRequest, GetHeadersResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_headers<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: GetHeadersRequest,
) -> Result<GetHeadersResponse, NodeError> {
    let context = context.read().await;
    let count = std::cmp::min(context.opts.max_blocks_fetch, req.count);
    Ok(GetHeadersResponse {
        headers: context.blockchain.get_headers(req.since, count)?,
    })
}
