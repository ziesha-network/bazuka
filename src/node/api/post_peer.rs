use super::messages::{PostPeerRequest, PostPeerResponse};
use super::{NodeContext, NodeError, Peer};
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
        .or_insert(Peer {
            address: req.address,
            info: Some(req.info),
            punished_until: 0,
        });
    Ok(PostPeerResponse {
        info: context.get_info()?,
        timestamp: context.network_timestamp(),
    })
}
