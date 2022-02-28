use super::messages::{GetPeersRequest, GetPeersResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_peers<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    _req: GetPeersRequest,
) -> Result<GetPeersResponse, NodeError> {
    let context = context.read().await;
    Ok(GetPeersResponse {
        peers: context
            .peers
            .clone()
            .into_iter()
            .filter_map(|(k, v)| if let Some(v) = v { Some((k, v)) } else { None })
            .collect(),
    })
}
