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
        social_profiles: context.social_profiles.clone(),
        height: context.blockchain.get_height()?,
        active_peers: context.peer_manager.get_peers().len(),
        power: context.blockchain.get_power()?,
        next_reward: context.blockchain.next_reward()?,
        timestamp: context.network_timestamp(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
}
