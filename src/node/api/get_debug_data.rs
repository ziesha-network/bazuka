use super::messages::{GetDebugDataRequest, GetDebugDataResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::db::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_debug_data<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    _req: GetDebugDataRequest,
) -> Result<GetDebugDataResponse, NodeError> {
    let context = context.read().await;
    Ok(GetDebugDataResponse {
        logs: "".into(),
        db_checksum: context.blockchain.db_checksum()?,
    })
}
