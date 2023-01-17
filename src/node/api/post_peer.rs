use super::messages::{HandshakeRequest, HandshakeResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::utils;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn post_peer<B: Blockchain>(
    client: Option<SocketAddr>,
    context: Arc<RwLock<NodeContext<B>>>,
    req: HandshakeRequest,
) -> Result<HandshakeResponse, NodeError> {
    let mut context = context.write().await;
    if let HandshakeRequest::Node(peer) = req {
        if let Some(client) = client {
            // Requester and proposed peer should have same IP.
            // Prevents attacking and flooding the peer list!
            if client.ip() != peer.ip() {
                return Err(NodeError::HandshakeClientMismatch);
            }
        }
        context
            .peer_manager
            .add_candidate(utils::local_timestamp(), peer);
    }

    Ok(HandshakeResponse {
        peer: context.get_info()?.ok_or(NodeError::NodeIsClientOnly)?,
        timestamp: context.network_timestamp(),
    })
}
