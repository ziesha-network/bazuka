use super::messages::{PostPeerRequest, PostPeerResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::client::Peer;
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
            pub_key: None,
            address: req.address,
            info: Some(req.info),
        });
    Ok(PostPeerResponse {
        info: context.get_info()?,
        timestamp: context.network_timestamp(),
    })
}
