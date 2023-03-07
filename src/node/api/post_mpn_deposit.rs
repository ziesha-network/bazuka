use super::messages::{PostMpnDepositRequest, PostMpnDepositResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::core::ChainSourcedTx;
use crate::db::KvStore;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn post_mpn_deposit<K: KvStore, B: Blockchain<K>>(
    client: Option<SocketAddr>,
    context: Arc<RwLock<NodeContext<K, B>>>,
    req: PostMpnDepositRequest,
) -> Result<PostMpnDepositResponse, NodeError> {
    let mut context = context.write().await;
    let is_local = client.map(|c| c.ip().is_loopback()).unwrap_or(false);
    context.mempool_add_chain_sourced(is_local, ChainSourcedTx::MpnDeposit(req.tx))?;
    Ok(PostMpnDepositResponse {})
}
