use super::messages::{GetMpnWorkRequest, GetMpnWorkResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::db::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_mpn_work<K: KvStore, B: Blockchain<K>>(
    _context: Arc<RwLock<NodeContext<K, B>>>,
    _req: GetMpnWorkRequest,
) -> Result<GetMpnWorkResponse, NodeError> {
    Ok(GetMpnWorkResponse {})
}
