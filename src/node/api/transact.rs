use super::messages::{TransactRequest, TransactResponse};
use super::{NodeContext, NodeError, TransactionStats};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn transact<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: TransactRequest,
) -> Result<TransactResponse, NodeError> {
    let mut context = context.write().await;
    let now = context.network_timestamp();
    // Prevent spamming mempool
    if context.blockchain.validate_transaction(&req.tx_patch)? {
        context
            .mempool
            .insert(req.tx_patch, TransactionStats { first_seen: now });
    }
    Ok(TransactResponse {})
}
