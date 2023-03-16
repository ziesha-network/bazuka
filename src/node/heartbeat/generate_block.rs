use super::*;
use crate::core::{Amount, ChainSourcedTx, Money, MpnAddress, MpnSourcedTx, TokenId};
use crate::mpn;
use std::collections::HashSet;

pub async fn generate_block<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
) -> Result<(), NodeError> {
    let mut ctx = context.write().await;
    let timestamp = ctx.network_timestamp();
    let proof = ctx.blockchain.validator_status(timestamp, &ctx.wallet)?;
    if !proof.is_unproven() {
        if ctx.opts.automatic_block_generation {
            if let Some(work_pool) = &ctx.mpn_work_pool {
                let wallet = ctx.wallet.clone();
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
        let claim = ctx.wallet.claim_validator(timestamp, proof, node);
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
                .min_validator_reward(ctx.wallet.get_address())?;

            let nonce = ctx.blockchain.get_account(ctx.wallet.get_address())?.nonce;
            let mpn_nonce = ctx
                .blockchain
                .get_mpn_account(
                    MpnAddress {
                        pub_key: ctx.wallet.get_zk_address(),
                    }
                    .account_index(ctx.blockchain.config().mpn_config.log4_tree_size),
                )?
                .nonce;
            deposits.insert(
                0,
                ctx.wallet.deposit_mpn(
                    "".into(),
                    ctx.blockchain.config().mpn_config.mpn_contract_id,
                    MpnAddress {
                        pub_key: ctx.wallet.get_zk_address(),
                    },
                    0,
                    nonce + 2,
                    Money {
                        amount: validator_reward,
                        token_id: TokenId::Ziesha,
                    },
                    Money {
                        amount: 0.into(),
                        token_id: TokenId::Ziesha,
                    },
                ),
            );
            ctx.mpn_work_pool = Some(mpn::prepare_works(
                &ctx.blockchain.config().mpn_config,
                ctx.blockchain.database(),
                &ctx.mpn_workers,
                &deposits,
                &withdraws,
                &updates,
                Amount(100_000_000_000), // TODO: Remove Hardcoded rewards
                Amount(100_000_000_000),
                Amount(300_000_000_000),
                mpn_nonce,
                ctx.wallet.clone(),
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
