use super::messages::{GetDelegationsRequest, GetDelegationsResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::db::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_delegations<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
    req: GetDelegationsRequest,
) -> Result<GetDelegationsResponse, NodeError> {
    let context = context.read().await;
    Ok(GetDelegationsResponse {
        delegatees: context
            .blockchain
            .get_delegatees(req.address.parse()?, Some(req.top))?
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
        delegators: context
            .blockchain
            .get_delegators(req.address.parse()?, Some(req.top))?
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    })
}
