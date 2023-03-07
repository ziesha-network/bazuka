use super::messages::{ShutdownRequest, ShutdownResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::node::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn shutdown<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
    _req: ShutdownRequest,
) -> Result<ShutdownResponse, NodeError> {
    context.write().await.shutdown = true;
    Ok(ShutdownResponse {})
}
