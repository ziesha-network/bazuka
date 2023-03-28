use super::messages::{TransactRequest, TransactResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::{Blockchain, BlockchainError};
use crate::core::ChainSourcedTx;
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
    match req {
        TransactRequest::ChainSourcedTx(chain_sourced_tx) => {
            let error = if let ChainSourcedTx::TransactionAndDelta(tx_delta) = &chain_sourced_tx {
                ctx.blockchain
                    .check_tx(&tx_delta.tx)
                    .err()
                    .filter(|e| !matches!(e, BlockchainError::InvalidTransactionNonce))
                    .map(|e| e.to_string())
            } else {
                None
            };
            if error.is_none() {
                let is_local = client.map(|c| c.ip().is_loopback()).unwrap_or(false);
                ctx.mempool_add_chain_sourced(is_local, chain_sourced_tx)?;
            }
            Ok(TransactResponse { error })
        }
        TransactRequest::MpnSourcedTx(mpn_sourced_tx) => {
            let is_local = client.map(|c| c.ip().is_loopback()).unwrap_or(false);
            ctx.mempool_add_mpn_sourced(is_local, mpn_sourced_tx)?;
            Ok(TransactResponse { error: None })
        }
    }
}
