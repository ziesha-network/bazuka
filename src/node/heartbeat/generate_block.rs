use super::*;
use crate::core::{Amount, ChainSourcedTx, MpnAddress, MpnSourcedTx};
use crate::mpn;
use std::collections::HashSet;

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
                    ctx.mempool_add_chain_sourced(
                        true,
                        ChainSourcedTx::TransactionAndDelta(tx_delta),
                    )?;
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

            let mut updates = Vec::new();
            let mut deposits = Vec::new();
            let mut withdraws = Vec::new();

            let mut has_tx_delta_before_mpn = HashSet::new();
            let mut chain_sourced_sorted = mempool
                .chain_sourced()
                .map(|(tx, _)| tx.clone())
                .collect::<Vec<_>>();
            chain_sourced_sorted.sort_unstable_by_key(|t| t.nonce());
            chain_sourced_sorted = ctx
                .blockchain
                .cleanup_chain_mempool(&chain_sourced_sorted)?;
            for tx in chain_sourced_sorted {
                match tx {
                    ChainSourcedTx::MpnDeposit(mpn_dep) => {
                        if !has_tx_delta_before_mpn.contains(&mpn_dep.payment.src) {
                            deposits.push(mpn_dep.clone());
                        }
                    }
                    ChainSourcedTx::TransactionAndDelta(tx_delta) => {
                        // Make sure there are no regular transactions before any MpnDeposit
                        // Since MPN-update transaction comes first, processing any regular
                        // transaction could invalidate MPN-update (Because of the invalid nonce)
                        // TODO: Is there a better solution?
                        has_tx_delta_before_mpn.insert(tx_delta.tx.src.clone().unwrap_or_default());
                    }
                }
            }

            let mut mpn_sourced_sorted = mempool
                .mpn_sourced()
                .map(|(tx, _)| tx.clone())
                .collect::<Vec<_>>();
            mpn_sourced_sorted.sort_unstable_by_key(|t| t.nonce());
            for tx in mpn_sourced_sorted {
                match &tx {
                    MpnSourcedTx::MpnTransaction(mpn_tx) => {
                        updates.push(mpn_tx.clone());
                    }
                    MpnSourcedTx::MpnWithdraw(mpn_withdraw) => {
                        withdraws.push(mpn_withdraw.clone());
                    }
                }
            }

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
