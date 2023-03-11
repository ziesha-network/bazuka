use super::messages::{GetDelegateesRequest, GetDelegateesResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::db::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_delegatees<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
    req: GetDelegateesRequest,
) -> Result<GetDelegateesResponse, NodeError> {
    let context = context.read().await;
    Ok(GetDelegateesResponse {
        delegatees: context
            .blockchain
            .get_delegatees(req.address.parse()?, req.top)?
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    })
}
