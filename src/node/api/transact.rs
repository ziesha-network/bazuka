use super::messages::{TransactRequest, TransactResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::{Blockchain, TransactionStats};
use crate::core::ChainSourcedTx;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn transact<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: TransactRequest,
) -> Result<TransactResponse, NodeError> {
    let mut context = context.write().await;
    let now = context.local_timestamp();
    context.mempool.chain_sourced.insert(
        ChainSourcedTx::TransactionAndDelta(req.tx_delta),
        TransactionStats::new(now),
    );
    Ok(TransactResponse {})
}
