use super::messages::{GetZeroMempoolRequest, GetZeroMempoolResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::core::{ChainSourcedTx, MpnSourcedTx};
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
        for tx in context.mempool.chain_sourced.keys() {
            if let ChainSourcedTx::MpnDeposit(mpn_dep) = tx {
                deposits.push(mpn_dep.clone());
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
