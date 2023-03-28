use super::messages::{PostMpnTransactionRequest, PostMpnTransactionResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::core::MpnSourcedTx;
use crate::db::KvStore;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn post_mpn_transaction<B: Blockchain>(
    client: Option<SocketAddr>,
    context: Arc<RwLock<NodeContext<B>>>,
    req: PostMpnTransactionRequest,
) -> Result<PostMpnTransactionResponse, NodeError> {
    let mut context = context.write().await;
    let is_local = client.map(|c| c.ip().is_loopback()).unwrap_or(false);
    context.mempool_add_mpn_sourced(is_local, MpnSourcedTx::MpnTransaction(req.tx))?;
    Ok(PostMpnTransactionResponse {})
}
