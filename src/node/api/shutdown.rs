use super::messages::{ShutdownRequest, ShutdownResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn shutdown<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    _req: ShutdownRequest,
) -> Result<ShutdownResponse, NodeError> {
    context.write().await.shutdown = true;
    Ok(ShutdownResponse {})
}
