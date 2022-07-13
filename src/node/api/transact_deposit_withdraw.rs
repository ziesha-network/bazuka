use super::messages::{TransactDepositWithdrawRequest, TransactDepositWithdrawResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::{Blockchain, TransactionStats};
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn transact_deposit_withdraw<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: TransactDepositWithdrawRequest,
) -> Result<TransactDepositWithdrawResponse, NodeError> {
    let mut context = context.write().await;
    let now = context.network_timestamp();
    // Prevent spamming mempool
    if context.blockchain.validate_dw_transaction(&req.tx)? {
        context
            .dw_mempool
            .insert(req.tx, TransactionStats { first_seen: now });
    }
    Ok(TransactDepositWithdrawResponse {})
}
