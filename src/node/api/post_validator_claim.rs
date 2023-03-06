use super::messages::{PostValidatorClaimRequest, PostValidatorClaimResponse};
use super::{promote_validator_claim, NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::db::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn post_validator_claim<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
    req: PostValidatorClaimRequest,
) -> Result<PostValidatorClaimResponse, NodeError> {
    let mut ctx = context.write().await;
    if ctx.update_validator_claim(req.validator_claim.clone())? {
        drop(ctx);
        promote_validator_claim(context, req.validator_claim).await;
    }
    Ok(PostValidatorClaimResponse {})
}
