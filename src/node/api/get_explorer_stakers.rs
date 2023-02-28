use super::messages::{GetExplorerStakersRequest, GetExplorerStakersResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_explorer_stakers<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    _req: GetExplorerStakersRequest,
) -> Result<GetExplorerStakersResponse, NodeError> {
    let context = context.read().await;
    let ts = context.network_timestamp();
    let (epoch, _) = context.blockchain.epoch_slot(ts);
    let current = context.blockchain.get_stakers(epoch)?;
    let next = context.blockchain.get_stakers(epoch + 1)?;
    Ok(GetExplorerStakersResponse {
        current: current.iter().map(|b| b.into()).collect(),
        next: next.iter().map(|b| b.into()).collect(),
    })
}
