use super::messages::{PostMpnSolutionRequest, PostMpnSolutionResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::db::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn post_mpn_solution<K: KvStore, B: Blockchain<K>>(
    _context: Arc<RwLock<NodeContext<K, B>>>,
    _req: PostMpnSolutionRequest,
) -> Result<PostMpnSolutionResponse, NodeError> {
    Ok(PostMpnSolutionResponse {})
}
