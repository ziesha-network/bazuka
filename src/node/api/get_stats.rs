use super::messages::{GetStatsRequest, GetStatsResponse};
use super::{Network, NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_stats<B: Blockchain, N: Network>(
    context: Arc<RwLock<NodeContext<N, B>>>,
    _req: GetStatsRequest,
) -> Result<GetStatsResponse, NodeError> {
    let context = context.read().await;
    Ok(GetStatsResponse {
        height: context.blockchain.get_height()?,
        power: context.blockchain.get_power()?,
        next_reward: context.blockchain.next_reward()?,
    })
}
