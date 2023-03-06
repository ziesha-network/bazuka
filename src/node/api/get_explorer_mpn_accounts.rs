use super::messages::{GetExplorerMpnAccountsRequest, GetExplorerMpnAccountsResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::db::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_explorer_mpn_accounts<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
    req: GetExplorerMpnAccountsRequest,
) -> Result<GetExplorerMpnAccountsResponse, NodeError> {
    let context = context.read().await;
    Ok(GetExplorerMpnAccountsResponse {
        accounts: context
            .blockchain
            .get_mpn_accounts(req.page, req.page_size)?
            .into_iter()
            .map(|(ind, a)| (ind, (&a).into()))
            .collect(),
    })
}
