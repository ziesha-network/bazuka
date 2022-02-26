use super::messages::{PostPeerRequest, PostPeerResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn post_peer<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: PostPeerRequest,
) -> Result<PostPeerResponse, NodeError> {
    let mut context = context.write().await;
    context.peers.insert(req.address, req.info);
    Ok(PostPeerResponse {})
}
