use super::messages::{GetMempoolRequest, GetMempoolResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_mempool<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    _req: GetMempoolRequest,
) -> Result<GetMempoolResponse, NodeError> {
    let mut context = context.write().await;
    context.refresh()?;
    Ok(GetMempoolResponse {
        tx: context.mempool.clone().into_keys().collect(),
        tx_zk: context
            .contract_payment_mempool
            .clone()
            .into_keys()
            .collect(),
        zk: context.zero_mempool.clone().into_keys().collect(),
    })
}
