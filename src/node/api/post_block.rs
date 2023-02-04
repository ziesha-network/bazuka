use super::messages::{PostBlockRequest, PostBlockResponse};
use super::{http, Limit, NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::common::*;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn post_block<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: PostBlockRequest,
) -> Result<PostBlockResponse, NodeError> {
    let mut context = context.write().await;
    if req.block.header.number == context.blockchain.get_height()? {
        context
            .blockchain
            .extend(req.block.header.number, &[req.block.clone()])?;
        context.on_update()?;
        context.blockchain.update_states(&req.patch)?;
        let net = context.outgoing.clone();
        let peer_addresses = context.peer_manager.get_peers();
        drop(context);

        http::group_request(&peer_addresses, |peer| {
            net.bincode_post::<PostBlockRequest, PostBlockResponse>(
                format!("http://{}/bincode/blocks", peer.address),
                req.clone(),
                Limit::default().size(KB).time(3 * SECOND),
            )
        })
        .await;
    }
    Ok(PostBlockResponse {})
}
