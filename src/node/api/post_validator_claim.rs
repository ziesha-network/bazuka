use super::messages::{PostValidatorClaimRequest, PostValidatorClaimResponse};
use super::{promote_validator_claim, NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn post_validator_claim<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: PostValidatorClaimRequest,
) -> Result<PostValidatorClaimResponse, NodeError> {
    let mut ctx = context.write().await;
    let ts = ctx.network_timestamp();
    if ctx.blockchain.is_validator(
        ts,
        req.validator_claim.address.clone(),
        req.validator_claim.proof.clone(),
    )? && req.validator_claim.verify_signature()
    {
        if ctx.validator_claim != Some(req.validator_claim.clone()) {
            // Only handle one winner!
            if let Some(curr_claim) = ctx.validator_claim.clone() {
                let (epoch_curr, slot_curr) = ctx.blockchain.epoch_slot(curr_claim.timestamp);
                let (epoch_req, slot_req) =
                    ctx.blockchain.epoch_slot(req.validator_claim.timestamp);
                if epoch_curr == epoch_req && slot_curr == slot_req {
                    return Ok(PostValidatorClaimResponse {});
                }
            }

            ctx.validator_claim = Some(req.validator_claim.clone());
            drop(ctx);
            log::info!("Address {} is the validator!", req.validator_claim.address);
            promote_validator_claim(context, req.validator_claim).await;
        }
    }
    Ok(PostValidatorClaimResponse {})
}
