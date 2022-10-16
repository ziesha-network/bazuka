use super::messages::{PostMpnPaymentRequest, PostMpnPaymentResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::{Blockchain, TransactionStats};
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn post_mpn_payment<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: PostMpnPaymentRequest,
) -> Result<PostMpnPaymentResponse, NodeError> {
    let mut context = context.write().await;
    let now = context.local_timestamp();
    // Prevent spamming mempool
    match context.blockchain.validate_mpn_payment(&req.tx) {
        Ok(_) => {
            context
                .mempool
                .tx_zk
                .insert(req.tx, TransactionStats { first_seen: now });
        }
        Err(e) => {
            log::warn!("Rejected contract payment. Error: {}", e);
        }
    }
    Ok(PostMpnPaymentResponse {})
}
