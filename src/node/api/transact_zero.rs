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
    let now = context.network_timestamp();
    // Prevent spamming mempool
    if context.blockchain.validate_zero_transaction(&req.tx)? {
        context
            .zero_mempool
            .insert(req.tx, TransactionStats { first_seen: now });
    }
    Ok(TransactZeroResponse {})
}
