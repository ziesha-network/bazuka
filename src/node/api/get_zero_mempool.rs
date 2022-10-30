use super::messages::{GetZeroMempoolRequest, GetZeroMempoolResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
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
        Ok(GetZeroMempoolResponse {
            updates: context.mempool.zk.clone().into_keys().collect(),
            deposits: context.mempool.tx_zk.clone().into_keys().collect(),
            withdraws: context.mempool.zk_tx.clone().into_keys().collect(),
        })
    }
}
