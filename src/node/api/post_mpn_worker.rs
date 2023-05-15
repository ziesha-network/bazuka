use super::messages::{PostMpnWorkerRequest, PostMpnWorkerResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::db::KvStore;
use crate::mpn::MpnWorker;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn post_mpn_worker<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
    req: PostMpnWorkerRequest,
) -> Result<PostMpnWorkerResponse, NodeError> {
    let mut context = context.write().await;
    if context.mpn_workers.contains_key(&req.address) {
        context.mpn_workers.insert(
            req.address.clone(),
            MpnWorker {
                address: req.address,
            },
        );
        Ok(PostMpnWorkerResponse { accepted: true })
    } else {
        Ok(PostMpnWorkerResponse { accepted: false })
    }
}
