use super::messages::{PostMpnWorkerRequest, PostMpnWorkerResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::db::KvStore;
use crate::mpn::MpnWorker;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn post_mpn_worker<K: KvStore, B: Blockchain<K>>(
    client: Option<SocketAddr>,
    context: Arc<RwLock<NodeContext<K, B>>>,
    req: PostMpnWorkerRequest,
) -> Result<PostMpnWorkerResponse, NodeError> {
    let mut context = context.write().await;
    let ip = client.map(|c| c.ip()).ok_or(NodeError::SenderIpUnknown)?;
    context.mpn_workers.insert(
        ip,
        MpnWorker {
            mpn_address: req.mpn_address,
        },
    );
    Ok(PostMpnWorkerResponse { accepted: true })
}
