use super::messages::{PostPeerRequest, PostPeerResponse};
use super::{NodeContext, NodeError, PeerStats};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn post_peer<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: PostPeerRequest,
) -> Result<PostPeerResponse, NodeError> {
    let mut context = context.write().await;
    context
        .peers
        .entry(req.address)
        .and_modify(|s| {
            s.info = Some(req.info.clone());
        })
        .or_insert(PeerStats {
            info: Some(req.info),
            punished_until: None,
        });
    Ok(PostPeerResponse {
        info: context.get_info()?,
        timestamp: context.network_timestamp(),
    })
}
