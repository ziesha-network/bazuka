use super::messages::{GetPeersRequest, GetPeersResponse};
use super::{Network, NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_peers<B: Blockchain, N: Network>(
    context: Arc<RwLock<NodeContext<N, B>>>,
    _req: GetPeersRequest,
) -> Result<GetPeersResponse, NodeError> {
    let context = context.read().await;
    Ok(GetPeersResponse {
        peers: context.active_peers(),
    })
}
