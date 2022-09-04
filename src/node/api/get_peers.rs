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
    let num_peers = context.opts.num_peers;
    let peers = context
        .random_peers(&mut rand::thread_rng(), num_peers)
        .into_iter()
        .map(|p| p.address)
        .collect();
    Ok(GetPeersResponse { peers })
}
