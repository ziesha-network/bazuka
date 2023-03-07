use super::messages::{GetDebugDataRequest, GetDebugDataResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::db::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_debug_data<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
    _req: GetDebugDataRequest,
) -> Result<GetDebugDataResponse, NodeError> {
    let context = context.read().await;
    Ok(GetDebugDataResponse {
        logs: "".into(),
        db_checksum: context.blockchain.db_checksum()?,
    })
}
