use super::messages::{PostMpnSolutionRequest, PostMpnSolutionResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn post_mpn_solution<B: Blockchain>(
    _context: Arc<RwLock<NodeContext<B>>>,
    _req: PostMpnSolutionRequest,
) -> Result<PostMpnSolutionResponse, NodeError> {
    Ok(PostMpnSolutionResponse {})
}
