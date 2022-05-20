use super::messages::{GetStatsRequest, GetStatsResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_stats<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    _req: GetStatsRequest,
) -> Result<GetStatsResponse, NodeError> {
    let context = context.read().await;
    Ok(GetStatsResponse {
        height: context.blockchain.get_height()?,
        state_height: context.blockchain.get_state_height()?,
        power: context.blockchain.get_power()?,
        next_reward: context.blockchain.next_reward()?,
        timestamp: context.network_timestamp(),
    })
}
