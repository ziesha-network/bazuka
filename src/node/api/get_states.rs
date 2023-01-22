use super::messages::{GetStatesRequest, GetStatesResponse, InputError};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::core::{hash::Hash, Hasher};
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_states<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: GetStatesRequest,
) -> Result<GetStatesResponse, NodeError> {
    let context = context.read().await;
    let to =
        <Hasher as Hash>::Output::try_from(hex::decode(req.to).map_err(|_| InputError::Invalid)?)
            .map_err(|_| InputError::Invalid)?;
    let patch = context
        .blockchain
        .generate_state_patch(req.outdated_heights, to)?;
    Ok(GetStatesResponse { patch })
}
