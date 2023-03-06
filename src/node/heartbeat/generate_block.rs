use super::*;
use crate::mpn;

pub async fn generate_block<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
) -> Result<(), NodeError> {
    let mut ctx = context.write().await;
    let timestamp = ctx.network_timestamp();
    let proof = ctx.blockchain.validator_status(timestamp, &ctx.wallet)?;
    if !proof.is_unproven() {
        if ctx.opts.automatic_block_generation {
            let wallet = ctx.wallet.clone();
            // Invoke PoS block generation
            if let Some(draft) = ctx.try_produce(wallet)? {
                drop(ctx);
                promote_block(context, draft).await;
                return Ok(());
            }
        }
        let node = ctx.address.ok_or(NodeError::ValidatorNotExposed)?;
        let claim = ctx.wallet.claim_validator(timestamp, proof, node);
        if ctx.update_validator_claim(claim.clone())? {
            ctx.mpn_work_pool = Some(mpn::prepare_works(
                &ctx.blockchain.config().mpn_config,
                ctx.blockchain.database(),
                &[],
                &[],
                &[],
            )?);
        }
        if let Some(claim) = ctx.validator_claim.clone() {
            println!("You are the validator! Promoting...");
            if claim.address == ctx.wallet.get_address() {
                drop(ctx);
                promote_validator_claim(context, claim).await;
            }
        }
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
