use super::messages::{PostMpnWithdrawRequest, PostMpnWithdrawResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::core::MpnSourcedTx;
use crate::db::KvStore;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn post_mpn_withdraw<K: KvStore, B: Blockchain<K>>(
    client: Option<SocketAddr>,
    context: Arc<RwLock<NodeContext<K, B>>>,
    req: PostMpnWithdrawRequest,
) -> Result<PostMpnWithdrawResponse, NodeError> {
    let mut context = context.write().await;
    let is_local = client.map(|c| c.ip().is_loopback()).unwrap_or(false);
    context.mempool_add_mpn_sourced(is_local, MpnSourcedTx::MpnWithdraw(req.tx))?;
    Ok(PostMpnWithdrawResponse {})
}
