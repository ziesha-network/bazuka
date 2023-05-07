use super::messages::{GetLogsRequest, GetLogsResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::db::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_logs<K: KvStore, B: Blockchain<K>>(
    _context: Arc<RwLock<NodeContext<K, B>>>,
    _req: GetLogsRequest,
) -> Result<GetLogsResponse, NodeError> {
    Ok(GetLogsResponse {
        logs: crate::GLOBAL_LOGS.lock().unwrap().clone(),
    })
}
