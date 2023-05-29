use super::messages::{TransactRequest, TransactResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::{Blockchain, BlockchainError};
use crate::core::GeneralTransaction;
use crate::db::KvStore;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn transact<K: KvStore, B: Blockchain<K>>(
    client: Option<SocketAddr>,
    context: Arc<RwLock<NodeContext<K, B>>>,
    req: TransactRequest,
) -> Result<TransactResponse, NodeError> {
    let mut ctx = context.write().await;

    if let GeneralTransaction::TransactionAndDelta(tx_delta) = &req.tx {
        if let Some(err) = ctx.blockchain.check_tx(&tx_delta.tx).err() {
            if !matches!(err, BlockchainError::InvalidTransactionNonce) {
                return Ok(TransactResponse {
                    error: Some(err.to_string()),
                });
            }
        }
    }

    let is_local = client.map(|c| c.ip().is_loopback()).unwrap_or(false);
    ctx.mempool_add_tx(is_local, req.tx, req.meta)?;
    Ok(TransactResponse { error: None })
}
