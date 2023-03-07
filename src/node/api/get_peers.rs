use super::messages::{GetPeersRequest, GetPeersResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::db::KvStore;
use rand::prelude::IteratorRandom;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_peers<K: KvStore, B: Blockchain<K>>(
    client: Option<SocketAddr>,
    context: Arc<RwLock<NodeContext<K, B>>>,
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
            .get_nodes()
            .choose_multiple(&mut rand::thread_rng(), num_peers)
            .into_iter()
            .map(|p| p.address)
            .collect(),
    })
}
