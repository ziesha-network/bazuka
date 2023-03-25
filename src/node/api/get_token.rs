use super::messages::{GetTokenInfoRequest, GetTokenInfoResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::db::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_token<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
    req: GetTokenInfoRequest,
) -> Result<GetTokenInfoResponse, NodeError> {
    let context = context.read().await;
    let token = context.blockchain.get_token(req.token_id.parse()?)?;
    Ok(GetTokenInfoResponse { token })
}
