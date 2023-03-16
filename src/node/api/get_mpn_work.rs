use super::messages::{GetMpnWorkRequest, GetMpnWorkResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::db::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_mpn_work<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
    req: GetMpnWorkRequest,
) -> Result<GetMpnWorkResponse, NodeError> {
    let ctx = context.read().await;
    Ok(GetMpnWorkResponse {
        works: ctx
            .mpn_work_pool
            .as_ref()
            .map(|p| p.get_works(req.mpn_address))
            .unwrap_or_default(),
    })
}
