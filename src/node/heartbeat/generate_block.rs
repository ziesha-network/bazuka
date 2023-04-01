use super::*;
use crate::core::{Amount, MpnAddress};
use crate::mpn;

pub async fn generate_block<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
) -> Result<(), NodeError> {
    let mut ctx = context.write().await;
    let timestamp = ctx.network_timestamp();
    let proof = ctx
        .blockchain
        .validator_status(timestamp, &ctx.validator_wallet)?;
    if !proof.is_unproven() {
        if ctx.opts.automatic_block_generation {
            if let Some(work_pool) = &ctx.mpn_work_pool {
                let wallet = ctx.validator_wallet.clone();
                let nonce = ctx.blockchain.get_account(wallet.get_address())?.nonce;
                if let Some(tx_delta) = work_pool.ready(&wallet, nonce + 1) {
                    log::info!("All MPN-proofs ready!");
                    ctx.mempool_add_tx(true, tx_delta.into())?;
                    if let Some(draft) = ctx.try_produce(wallet)? {
                        ctx.mpn_work_pool = None;
                        drop(ctx);
                        promote_block(context, draft).await;
                        return Ok(());
                    }
                }
            }
        }
        let node = ctx.address.ok_or(NodeError::ValidatorNotExposed)?;
        let claim = ctx.validator_wallet.claim_validator(timestamp, proof, node);
        if ctx.update_validator_claim(claim.clone())? {
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

            let nonce = ctx
                .blockchain
                .get_account(ctx.validator_wallet.get_address())?
                .nonce;
            let mpn_nonce = ctx
                .blockchain
                .get_mpn_account(
                    MpnAddress {
                        pub_key: ctx.validator_wallet.get_zk_address(),
                    }
                    .account_index(ctx.blockchain.config().mpn_config.log4_tree_size),
                )?
                .nonce;
            ctx.mpn_work_pool = Some(mpn::prepare_works(
                &ctx.blockchain.config().mpn_config,
                ctx.blockchain.database(),
                &ctx.mpn_workers,
                deposits,
                withdraws,
                updates,
                validator_reward,
                Amount(100_000_000_000), // TODO: Remove Hardcoded rewards
                Amount(100_000_000_000),
                Amount(300_000_000_000),
                nonce,
                mpn_nonce,
                ctx.validator_wallet.clone(),
                ctx.user_wallet.clone(),
            )?);
        }
        if let Some(claim) = ctx.validator_claim.clone() {
            println!("You are the validator! Promoting...");
            if claim.address == ctx.validator_wallet.get_address() {
                drop(ctx);
                promote_validator_claim(context, claim).await;
            }
        }
    } else {
        if let Some(claim) = ctx.validator_claim.clone() {
            if claim.address == ctx.validator_wallet.get_address() {
                if let Some(work_pool) = &ctx.mpn_work_pool {
                    for work in work_pool.remaining_works().values() {
                        log::error!("Prover {} is late!", work.worker.mpn_address);
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
