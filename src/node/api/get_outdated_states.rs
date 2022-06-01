use super::messages::{GetOutdatedStatesRequest, GetOutdatedStatesResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_outdated_states<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    _req: GetOutdatedStatesRequest,
) -> Result<GetOutdatedStatesResponse, NodeError> {
    let context = context.read().await;
    let outdated_states = context.blockchain.get_outdated_states_request()?;
    Ok(GetOutdatedStatesResponse { outdated_states })
}
