use super::messages::{GetBalanceRequest, GetBalanceResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::core::TokenId;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_balance<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: GetBalanceRequest,
) -> Result<GetBalanceResponse, NodeError> {
    let context = context.read().await;
    Ok(GetBalanceResponse {
        balance: context
            .blockchain
            .get_balance(req.address.parse()?, req.token_id.parse()?)?,
    })
}
