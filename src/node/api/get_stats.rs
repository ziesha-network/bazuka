use super::messages::{GetStatsRequest, GetStatsResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::db::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_stats<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
    _req: GetStatsRequest,
) -> Result<GetStatsResponse, NodeError> {
    let context = context.read().await;
    let ts = context.network_timestamp();
    let (epoch, slot) = context.blockchain.epoch_slot(ts);
    let randomness = context.blockchain.epoch_randomness()?;
    let random_number: f32 = context
        .validator_wallet
        .generate_random(randomness, slot)
        .0
        .into();

    Ok(GetStatsResponse {
        social_profiles: context.social_profiles.clone(),
        address: context.validator_wallet.get_address().to_string(),
        height: context.blockchain.get_height()?,
        nodes: context.peer_manager.node_count(),
        next_reward: context.blockchain.next_reward()?,
        timestamp: ts,
        timestamp_offset: context.timestamp_offset,
        epoch,
        slot,
        version: env!("CARGO_PKG_VERSION").into(),
        network: context.network.clone(),
        validator_claim: context.validator_claim.clone(),
        random_number,
    })
}
