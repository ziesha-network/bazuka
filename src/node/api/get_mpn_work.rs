use super::messages::{GetMpnWorkRequest, GetMpnWorkResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::db::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_mpn_work<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: GetMpnWorkRequest,
) -> Result<GetMpnWorkResponse, NodeError> {
    let ctx = context.read().await;
    let works = ctx
        .mpn_work_pool
        .as_ref()
        .map(|p| p.get_works(req.mpn_address.clone()))
        .unwrap_or_default();
    if !works.is_empty() {
        println!("Sending {} works to {}", works.len(), req.mpn_address);
    }
    Ok(GetMpnWorkResponse { works })
}
