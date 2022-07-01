use super::messages::{GetOutdatedHeightsRequest, GetOutdatedHeightsResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_outdated_heights<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    _req: GetOutdatedHeightsRequest,
) -> Result<GetOutdatedHeightsResponse, NodeError> {
    let context = context.read().await;
    Ok(GetOutdatedHeightsResponse {
        outdated_heights: context.blockchain.get_outdated_heights()?,
    })
}
