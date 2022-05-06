use super::messages::{GetHeadersRequest, GetHeadersResponse};
use super::{Network, NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_headers<B: Blockchain, N: Network>(
    context: Arc<RwLock<NodeContext<N, B>>>,
    req: GetHeadersRequest,
) -> Result<GetHeadersResponse, NodeError> {
    let context = context.read().await;
    Ok(GetHeadersResponse {
        headers: context.blockchain.get_headers(req.since, req.until)?,
    })
}
