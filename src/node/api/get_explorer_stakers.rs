use super::messages::{GetExplorerStakersRequest, GetExplorerStakersResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_explorer_stakers<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: GetExplorerStakersRequest,
) -> Result<GetExplorerStakersResponse, NodeError> {
    let context = context.read().await;
    let stakers = context.blockchain.get_stakers()?;
    Ok(GetExplorerStakersResponse {
        stakers: stakers.iter().map(|b| b.into()).collect(),
    })
}
