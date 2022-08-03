use super::messages::{GetMpnAccountRequest, GetMpnAccountResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::zk::MpnAccount;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_mpn_account<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: GetMpnAccountRequest,
) -> Result<GetMpnAccountResponse, NodeError> {
    let context = context.read().await;
    Ok(GetMpnAccountResponse {
        account: MpnAccount::default(),
    })
}
