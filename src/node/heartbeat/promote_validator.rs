use super::*;

pub async fn promote_validator<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    let mut ctx = context.write().await;
    let timestamp = ctx.network_timestamp();
    let proof = ctx.blockchain.validator_status(timestamp, &ctx.wallet)?;
    if !proof.is_unproven() {
        println!("You are the validator! Promoting...");
        if let Some(claim) = ctx.validator_claim.clone() {
            let (curr_epoch, curr_slot) = ctx.blockchain.epoch_slot(timestamp);
            let (claim_epoch, claim_slot) = ctx.blockchain.epoch_slot(claim.timestamp);
            let same_epoch_slot = curr_epoch == claim_epoch && curr_slot == claim_slot;
            if claim.address == ctx.wallet.get_address() && same_epoch_slot {
                drop(ctx);
                promote_validator_claim(context, claim).await;
                return Ok(());
            }
        }

        let node = ctx.address.ok_or(NodeError::ValidatorNotExposed)?;
        let claim = ctx.wallet.claim_validator(timestamp, proof, node);
        ctx.validator_claim = Some(claim.clone());
        drop(ctx);
        promote_validator_claim(context, claim).await;
    } else {
        ctx.mpn_work_pool = None;
        if let Some(claim) = ctx.validator_claim.clone() {
            if !ctx.blockchain.is_validator(
                timestamp,
                claim.address.clone(),
                claim.proof.clone(),
            )? {
                println!("{} is not the validator anymore!", claim.address);
                ctx.validator_claim = None;
            }
        }
    }
    Ok(())
}
