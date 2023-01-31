use super::messages::{GetTokenInfoRequest, GetTokenInfoResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::{Blockchain, BlockchainError};
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_token<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: GetTokenInfoRequest,
) -> Result<GetTokenInfoResponse, NodeError> {
    let context = context.read().await;
    let token = context
        .blockchain
        .get_token(req.token_id.parse()?)?
        .ok_or(BlockchainError::TokenNotFound)?;
    Ok(GetTokenInfoResponse { token })
}
