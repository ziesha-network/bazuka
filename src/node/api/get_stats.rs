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
    let ts = context.network_timestamp();
    let (epoch, slot) = context.blockchain.epoch_slot(ts);
    Ok(GetStatsResponse {
        social_profiles: context.social_profiles.clone(),
        address: context.wallet.get_address().to_string(),
        height: context.blockchain.get_height()?,
        nodes: context.peer_manager.node_count(),
        next_reward: context.blockchain.next_reward()?,
        timestamp: ts,
        epoch,
        slot,
        version: env!("CARGO_PKG_VERSION").into(),
        network: context.network.clone(),
        validator_proof: context.blockchain.validator_status(ts, &context.wallet)?,
        validator_claim: context.validator_claim.clone(),
    })
}
