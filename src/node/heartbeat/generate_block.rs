use super::*;
use crate::core::Amount;
use crate::mpn;

pub async fn generate_block<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
) -> Result<(), NodeError> {
    let mut ctx = context.write().await;
    let timestamp = ctx.network_timestamp();
    let proof = ctx
        .blockchain
        .validator_status(timestamp, &ctx.validator_wallet)?;

    if let Some(proof) = proof {
        let (tip_epoch, tip_slot) = ctx
            .blockchain
            .epoch_slot(ctx.blockchain.get_tip()?.proof_of_stake.timestamp);
        let (curr_epoch, curr_slot) = ctx.blockchain.epoch_slot(timestamp);
        if [curr_epoch, curr_slot] <= [tip_epoch, tip_slot] {
            return Ok(());
        }

        let node = ctx.address.ok_or(NodeError::ValidatorNotExposed)?;
        let claim = ctx.validator_wallet.claim_validator(timestamp, proof, node);
        if ctx.update_validator_claim(claim.clone())? {
            if ctx.opts.automatic_block_generation {
                let mempool = ctx.mempool.clone();

                let updates = mempool
                    .mpn_txs()
                    .map(|(tx, _)| tx.clone())
                    .collect::<Vec<_>>();
                let deposits = mempool
                    .mpn_deposits()
                    .map(|(tx, _)| tx.clone())
                    .collect::<Vec<_>>();
                let withdraws = mempool
                    .mpn_withdraws()
                    .map(|(tx, _)| tx.clone())
                    .collect::<Vec<_>>();

                let validator_reward = ctx
                    .blockchain
                    .min_validator_reward(ctx.validator_wallet.get_address())?;

                let deposit_nonce = ctx.blockchain.get_deposit_nonce(
                    ctx.validator_wallet.get_address(),
                    ctx.blockchain.config().mpn_config.mpn_contract_id,
                )?;
                ctx.mpn_work_pool = Some(mpn::prepare_works(
                    &ctx.blockchain.config().mpn_config,
                    &ctx.blockchain,
                    &ctx.mpn_workers,
                    deposits,
                    withdraws,
                    updates,
                    validator_reward,
                    Amount(100_000_000_000), // TODO: Remove Hardcoded rewards
                    Amount(100_000_000_000),
                    Amount(300_000_000_000),
                    deposit_nonce,
                    ctx.validator_wallet.clone(),
                    ctx.user_wallet.clone(),
                )?);
            }
        }
        if let Some(work_pool) = &ctx.mpn_work_pool {
            let wallet = ctx.validator_wallet.clone();
            let nonce = ctx.blockchain.get_nonce(wallet.get_address())?;
            if let Some(tx_delta) = work_pool.ready(&wallet, nonce + 1) {
                log::info!("All MPN-proofs ready!");
                ctx.mempool_add_tx(true, tx_delta.into())?;
                if let Some(draft) = ctx.try_produce(wallet)? {
                    ctx.mpn_work_pool = None;
                    ctx.validator_claim = None;
                    drop(ctx);
                    promote_block(context.clone(), draft).await;
                    return Ok(());
                }
            }
        } else {
            drop(ctx);
            promote_validator_claim(context.clone(), claim).await;
        }
    } else {
        if let Some(claim) = ctx.validator_claim.clone() {
            if claim.address == ctx.validator_wallet.get_address() {
                if let Some(work_pool) = &ctx.mpn_work_pool {
                    for work in work_pool.remaining_works().keys() {
                        log::error!("Solution for work {} is late!", work);
                    }
                }
            }
        }
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
