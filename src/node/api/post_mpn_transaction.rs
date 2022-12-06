use super::messages::{PostMpnTransactionRequest, PostMpnTransactionResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::{Blockchain, TransactionStats};
use crate::core::MpnSourcedTx;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn post_mpn_transaction<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: PostMpnTransactionRequest,
) -> Result<PostMpnTransactionResponse, NodeError> {
    let mut context = context.write().await;
    let now = context.local_timestamp();
    context.mempool.mpn_sourced.insert(
        MpnSourcedTx::MpnTransaction(req.tx),
        TransactionStats { first_seen: now },
    );
    Ok(PostMpnTransactionResponse {})
}
