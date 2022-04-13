use super::messages::{RegisterMinerRequest, RegisterMinerResponse};
use super::{Miner, NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn post_miner<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: RegisterMinerRequest,
) -> Result<RegisterMinerResponse, NodeError> {
    let mut context = context.write().await;
    context.miner = Some(Miner {
        webhook: req.webhook,
    });
    Ok(RegisterMinerResponse {})
}
