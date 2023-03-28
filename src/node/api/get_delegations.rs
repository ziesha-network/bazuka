use super::messages::{GetDelegationsRequest, GetDelegationsResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_delegations<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
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
