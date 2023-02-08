use super::messages::{TransactRequest, TransactResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::{Blockchain, TransactionStats};
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
        context.get_puzzle(wallet)?; // TODO: Invoke PoS block generation
    }
    Ok(TransactResponse {})
}
