use super::messages::{PostMpnTransactionRequest, PostMpnTransactionResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::{Blockchain, TransactionStats};
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn post_mpn_transaction<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: PostMpnTransactionRequest,
) -> Result<PostMpnTransactionResponse, NodeError> {
    let mut context = context.write().await;
    let now = context.local_timestamp();
    // Prevent spamming mempool
    match context.blockchain.validate_mpn_transaction(&req.tx) {
        Ok(_) => {
            context
                .mpn_tx_mempool
                .insert(req.tx, TransactionStats { first_seen: now });
        }
        Err(e) => {
            log::warn!("Rejected zero-transaction. Error: {}", e);
        }
    }
    Ok(PostMpnTransactionResponse {})
}
