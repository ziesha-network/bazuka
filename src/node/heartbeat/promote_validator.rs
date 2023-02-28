use super::*;

pub async fn promote_validator<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    let mut ctx = context.write().await;
    let timestamp = ctx.network_timestamp();
    let proof = ctx.blockchain.validator_status(timestamp, &ctx.wallet)?;
    if !proof.is_unproven() {
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
    } else if let Some(claim) = ctx.validator_claim.clone() {
        if !ctx
            .blockchain
            .is_validator(timestamp, claim.address.clone(), claim.proof.clone())?
        {
            println!("{} is not the validator anymore!", claim.address);
            ctx.validator_claim = None;
        }
    }
    Ok(())
}
