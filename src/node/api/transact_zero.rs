use super::messages::{TransactZeroRequest, TransactZeroResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::{Blockchain, TransactionStats};
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn transact_zero<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: TransactZeroRequest,
) -> Result<TransactZeroResponse, NodeError> {
    let mut context = context.write().await;
    let now = context.local_timestamp();
    // Prevent spamming mempool
    match context.blockchain.validate_zero_transaction(&req.tx) {
        Ok(_) => {
            context
                .zero_mempool
                .insert(req.tx, TransactionStats { first_seen: now });
        }
        Err(e) => {
            log::warn!("Rejected zero-transaction. Error: {}", e);
        }
    }
    Ok(TransactZeroResponse {})
}
