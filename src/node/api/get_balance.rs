use super::messages::{GetBalanceRequest, GetBalanceResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::core::TokenId;
use crate::db::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_balance<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
    req: GetBalanceRequest,
) -> Result<GetBalanceResponse, NodeError> {
    let context = context.read().await;
    let token_id: TokenId = req.token_id.parse()?;
    let tkn = context
        .blockchain
        .get_token(token_id)?
        .ok_or(crate::blockchain::BlockchainError::TokenNotFound)?;
    Ok(GetBalanceResponse {
        balance: context
            .blockchain
            .get_balance(req.address.parse()?, token_id)?,
        name: tkn.name,
        symbol: tkn.symbol,
    })
}
