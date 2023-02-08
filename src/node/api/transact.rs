use super::messages::{TransactRequest, TransactResponse};
use super::{http, NodeContext, NodeError};
use crate::blockchain::{Blockchain, TransactionStats};
use crate::client::messages::{PostBlockRequest, PostBlockResponse};
use crate::client::Limit;
use crate::common::*;
use crate::core::ChainSourcedTx;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn transact<B: Blockchain>(
    client: Option<SocketAddr>,
    context: Arc<RwLock<NodeContext<B>>>,
    req: TransactRequest,
) -> Result<TransactResponse, NodeError> {
    let mut context = context.write().await;
    let now = context.local_timestamp();
    let is_local = client.map(|c| c.ip().is_loopback()).unwrap_or(false);
    context.mempool.add_chain_sourced(
        ChainSourcedTx::TransactionAndDelta(req.tx_delta),
        TransactionStats::new(is_local, now),
    );
    if is_local {
        let wallet = context.wallet.clone();
        // Invoke PoS block generation
        if let Some(draft) = context.try_produce(wallet)? {
            let peer_addresses = context.peer_manager.get_peers();
            let net = context.outgoing.clone();
            drop(context);
            http::group_request(&peer_addresses, |peer| {
                net.bincode_post::<PostBlockRequest, PostBlockResponse>(
                    format!("http://{}/bincode/blocks", peer.address),
                    PostBlockRequest {
                        block: draft.block.clone(),
                        patch: draft.patch.clone(),
                    },
                    Limit::default().size(KB).time(3 * SECOND),
                )
            })
            .await;
        }
    }
    Ok(TransactResponse {})
}
