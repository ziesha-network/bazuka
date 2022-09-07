use super::messages::{HandshakeRequest, HandshakeResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn post_peer<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: HandshakeRequest,
) -> Result<HandshakeResponse, NodeError> {
    let mut context = context.write().await;
    if let HandshakeRequest::Node { address, peer } = req {
        context.peer_manager.add_peer(address, peer);
    }

    Ok(HandshakeResponse {
        peer: context.get_info()?.ok_or(NodeError::NodeIsClientOnly)?,
        timestamp: context.network_timestamp(),
    })
}
