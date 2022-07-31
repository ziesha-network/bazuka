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
    context.cleanup_mempools()?;
    Ok(GetZeroMempoolResponse {
        updates: context.zero_mempool.clone().into_keys().collect(),
        payments: context
            .contract_payment_mempool
            .clone()
            .into_keys()
            .collect(),
    })
}
