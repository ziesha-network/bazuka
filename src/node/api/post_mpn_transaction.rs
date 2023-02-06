use super::messages::{PostMpnTransactionRequest, PostMpnTransactionResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::{Blockchain, TransactionStats};
use crate::core::MpnSourcedTx;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn post_mpn_transaction<B: Blockchain>(
    client: Option<SocketAddr>,
    context: Arc<RwLock<NodeContext<B>>>,
    req: PostMpnTransactionRequest,
) -> Result<PostMpnTransactionResponse, NodeError> {
    let mut context = context.write().await;
    let now = context.local_timestamp();
    let is_local = client.map(|c| c.ip().is_loopback()).unwrap_or(false);
    if is_local || context.mempool.mpn_sourced().len() < context.opts.mpn_mempool_capacity {
        context.mempool.add_mpn_sourced(
            MpnSourcedTx::MpnTransaction(req.tx),
            TransactionStats::new(is_local, now),
        );
    }
    Ok(PostMpnTransactionResponse {})
}
