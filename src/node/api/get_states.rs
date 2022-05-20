use super::messages::{GetStatesRequest, GetStatesResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_states<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: GetStatesRequest,
) -> Result<GetStatesResponse, NodeError> {
    let context = context.read().await;
    let patch = context.blockchain.generate_state_patch(req.from, req.to)?;
    Ok(GetStatesResponse { patch })
}
