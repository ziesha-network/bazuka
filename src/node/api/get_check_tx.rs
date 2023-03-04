use super::messages::{CheckTransactionRequest, CheckTransactionResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::db::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_check_tx<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
    req: CheckTransactionRequest,
) -> Result<CheckTransactionResponse, NodeError> {
    let context = context.read().await;
    let error = context
        .blockchain
        .check_tx(&req.tx_delta.tx)
        .err()
        .map(|e| e.to_string());

    Ok(CheckTransactionResponse { error })
}
