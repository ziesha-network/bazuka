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
    let mut context = context.write().await;
    if context.blockchain.get_outdated_heights()?.len() > 0 {
        Err(NodeError::StatesOutdated)
    } else {
        context.refresh()?;
        let mut updates = Vec::new();
        let mut deposits = Vec::new();
        let mut withdraws = Vec::new();

        let mut has_tx_delta_before_mpn = HashSet::new();
        let mut chain_sourced_sorted = context.mempool.chain_sourced.keys().collect::<Vec<_>>();
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

        for tx in context.mempool.mpn_sourced.keys() {
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
