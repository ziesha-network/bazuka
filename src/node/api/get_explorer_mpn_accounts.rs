use super::messages::{GetExplorerMpnAccountsRequest, GetExplorerMpnAccountsResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_explorer_mpn_accounts<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
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
