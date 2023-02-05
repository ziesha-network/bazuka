use super::messages::{PostMpnWithdrawRequest, PostMpnWithdrawResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::{Blockchain, TransactionStats};
use crate::core::MpnSourcedTx;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn post_mpn_withdraw<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: PostMpnWithdrawRequest,
) -> Result<PostMpnWithdrawResponse, NodeError> {
    let mut context = context.write().await;
    let now = context.local_timestamp();
    context.mempool.mpn_sourced.insert(
        MpnSourcedTx::MpnWithdraw(req.tx),
        TransactionStats::new(now),
    );
    Ok(PostMpnWithdrawResponse {})
}
