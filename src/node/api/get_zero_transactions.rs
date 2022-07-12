use super::messages::{GetZeroTransactionsRequest, GetZeroTransactionsResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_zero_transactions<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    _req: GetZeroTransactionsRequest,
) -> Result<GetZeroTransactionsResponse, NodeError> {
    let _context = context.read().await;
    Ok(GetZeroTransactionsResponse {
        updates: vec![],
        deposit_withdraws: vec![],
    })
}
