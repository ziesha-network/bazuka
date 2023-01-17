use super::messages::{GetPeersRequest, GetPeersResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_peers<B: Blockchain>(
    client: Option<SocketAddr>,
    context: Arc<RwLock<NodeContext<B>>>,
    _req: GetPeersRequest,
) -> Result<GetPeersResponse, NodeError> {
    let context = context.read().await;
    if let Some(client) = client {
        if client.ip().is_loopback() {
            return Ok(GetPeersResponse {
                peers: context
                    .peer_manager
                    .get_nodes()
                    .map(|p| p.address)
                    .collect(),
            });
        }
    }
    let num_peers = context.opts.num_peers;
    Ok(GetPeersResponse {
        peers: context
            .peer_manager
            .get_peers(num_peers)
            .into_iter()
            .map(|p| p.address)
            .collect(),
    })
}
