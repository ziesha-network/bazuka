use super::messages::{GetAccountRequest, GetAccountResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_account<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: GetAccountRequest,
) -> Result<GetAccountResponse, NodeError> {
    let context = context.read().await;
    Ok(GetAccountResponse {
        account: context.blockchain.get_account(req.address.parse()?)?,
    })
}
