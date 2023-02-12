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
        let height = ctx.blockchain.get_height()?;
        let reward = ctx.blockchain.next_reward()?;
        let mut mempool = ctx.mempool.clone();
        ctx.blockchain.cleanup_chain_mempool(&mut mempool)?;
        drop(ctx);

        context.write().await.mempool = mempool.clone();

        let mut updates = Vec::new();
        let mut deposits = Vec::new();
        let mut withdraws = Vec::new();

        let mut has_tx_delta_before_mpn = HashSet::new();
        let mut chain_sourced_sorted = mempool
            .chain_sourced()
            .map(|(tx, _)| tx.clone())
            .collect::<Vec<_>>();
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

        let mut has_send_before_withdraw = HashSet::new();
        let mut mpn_sourced_sorted = mempool
            .mpn_sourced()
            .map(|(tx, _)| tx.clone())
            .collect::<Vec<_>>();
        mpn_sourced_sorted.sort_unstable_by_key(|t| t.nonce());
        for tx in mpn_sourced_sorted {
            match &tx {
                MpnSourcedTx::MpnTransaction(mpn_tx) => {
                    updates.push(mpn_tx.clone());
                    has_send_before_withdraw.insert(tx.sender());
                }
                MpnSourcedTx::MpnWithdraw(mpn_withdraw) => {
                    if !has_send_before_withdraw.contains(&tx.sender()) {
                        withdraws.push(mpn_withdraw.clone());
                    }
                }
            }
        }
        Ok(GetZeroMempoolResponse {
            height,
            reward,
            updates,
            deposits,
            withdraws,
        })
    }
}
