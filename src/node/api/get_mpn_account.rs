use super::messages::{GetMpnAccountRequest, GetMpnAccountResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::db::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_mpn_account<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
    req: GetMpnAccountRequest,
) -> Result<GetMpnAccountResponse, NodeError> {
    let context = context.read().await;
    Ok(GetMpnAccountResponse {
        account: context.blockchain.get_mpn_account(req.index)?,
    })
}
