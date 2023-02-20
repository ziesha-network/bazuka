use super::*;

pub async fn promote_validator<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    let mut ctx = context.write().await;
    let timestamp = ctx.network_timestamp();
    if let Some(proof) = ctx.blockchain.validator_status(timestamp, &ctx.wallet)? {
        if let Some(claim) = &ctx.validator_claim {
            if claim.address == ctx.wallet.get_address() {
                return Ok(());
            }
        }
        println!("You are the validator! Promoting...");
        let node = ctx.address.ok_or(NodeError::ValidatorNotExposed)?;
        let claim = ctx.wallet.claim_validator(timestamp, proof, node);
        ctx.validator_claim = Some(claim.clone());
        drop(ctx);
        promote_validator_claim(context, claim).await;
    } else if ctx.validator_claim.is_some() {
        println!("You are not the validator anymore!");
        ctx.validator_claim = None;
    }
    Ok(())
}
