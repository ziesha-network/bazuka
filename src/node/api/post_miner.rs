use super::messages::{RegisterMinerRequest, RegisterMinerResponse};
use super::{Miner, Network, NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn post_miner<B: Blockchain, N: Network>(
    context: Arc<RwLock<NodeContext<N, B>>>,
    req: RegisterMinerRequest,
) -> Result<RegisterMinerResponse, NodeError> {
    let mut context = context.write().await;
    println!("Registered miner!");
    context.miner = Some(Miner {
        webhook: req.webhook,
        block_puzzle: None,
    });
    Ok(RegisterMinerResponse {})
}
