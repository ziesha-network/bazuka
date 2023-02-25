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
    let ts = context.network_timestamp();
    let (epoch, _) = context.blockchain.epoch_slot(ts);
    let stakers = context.blockchain.get_stakers(epoch)?;
    Ok(GetExplorerStakersResponse {
        stakers: stakers.iter().map(|b| b.into()).collect(),
    })
}
