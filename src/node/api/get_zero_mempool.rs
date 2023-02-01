use super::messages::{GetZeroMempoolRequest, GetZeroMempoolResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::core::{ChainSourcedTx, MpnSourcedTx};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_zero_mempool<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    _req: GetZeroMempoolRequest,
) -> Result<GetZeroMempoolResponse, NodeError> {
    if !context
        .read()
        .await
        .blockchain
        .get_outdated_heights()?
        .is_empty()
    {
        Err(NodeError::StatesOutdated)
    } else {
        let ctx = context.read().await;
        let mut chain_sourced = ctx.mempool.chain_sourced.clone();
        let mut mpn_sourced = ctx.mempool.mpn_sourced.clone();
        ctx.blockchain.cleanup_chain_mempool(&mut chain_sourced)?;
        ctx.blockchain
            .cleanup_mpn_mempool(&mut mpn_sourced, ctx.opts.mpn_mempool_capacity)?;
        drop(ctx);

        let mut ctx = context.write().await;
        ctx.mempool.chain_sourced = chain_sourced.clone();
        ctx.mempool.mpn_sourced = mpn_sourced.clone();
        drop(ctx);

        let mut updates = Vec::new();
        let mut deposits = Vec::new();
        let mut withdraws = Vec::new();

        let mut has_tx_delta_before_mpn = HashSet::new();
        let mut chain_sourced_sorted = chain_sourced.keys().collect::<Vec<_>>();
        chain_sourced_sorted.sort_unstable_by_key(|t| t.nonce());
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

        for tx in mpn_sourced.keys() {
            match tx {
                MpnSourcedTx::MpnTransaction(mpn_tx) => {
                    updates.push(mpn_tx.clone());
                }
                MpnSourcedTx::MpnWithdraw(mpn_withdraw) => {
                    withdraws.push(mpn_withdraw.clone());
                }
            }
        }
        Ok(GetZeroMempoolResponse {
            updates,
            deposits,
            withdraws,
        })
    }
}
