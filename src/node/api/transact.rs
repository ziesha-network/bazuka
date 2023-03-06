use super::messages::{TransactRequest, TransactResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::core::ChainSourcedTx;
use crate::db::KvStore;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn transact<K: KvStore, B: Blockchain<K>>(
    client: Option<SocketAddr>,
    context: Arc<RwLock<NodeContext<K, B>>>,
    req: TransactRequest,
) -> Result<TransactResponse, NodeError> {
    let mut ctx = context.write().await;
    let is_local = client.map(|c| c.ip().is_loopback()).unwrap_or(false);
    ctx.mempool_add_chain_sourced(is_local, ChainSourcedTx::TransactionAndDelta(req.tx_delta))?;
    Ok(TransactResponse {})
}
