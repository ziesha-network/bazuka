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
    context
        .mempool
        .tx_zk
        .insert(req.tx, TransactionStats { first_seen: now });
    Ok(PostMpnPaymentResponse {})
}
