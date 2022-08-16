use super::messages::{TransactRequest, TransactResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::{Blockchain, TransactionStats};
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn transact<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: TransactRequest,
) -> Result<TransactResponse, NodeError> {
    let mut context = context.write().await;
    let now = context.network_timestamp();
    // Prevent spamming mempool
    match context.blockchain.validate_transaction(&req.tx_delta) {
        Ok(_) => {
            context
                .mempool
                .insert(req.tx_delta, TransactionStats { first_seen: now });
        }
        Err(e) => {
            log::warn!("Rejected transaction. Error: {}", e);
        }
    }
    Ok(TransactResponse {})
}
