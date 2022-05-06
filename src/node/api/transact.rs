use super::messages::{TransactRequest, TransactResponse};
use super::{Network, NodeContext, NodeError, TransactionStats};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn transact<B: Blockchain, N: Network>(
    context: Arc<RwLock<NodeContext<N, B>>>,
    req: TransactRequest,
) -> Result<TransactResponse, NodeError> {
    let mut context = context.write().await;
    let now = context.network_timestamp();
    // Prevent spamming mempool
    if context.blockchain.get_account(req.tx.src.clone())?.balance > 0 && req.tx.verify_signature()
    {
        context
            .mempool
            .insert(req.tx, TransactionStats { first_seen: now });
    }
    Ok(TransactResponse {})
}
