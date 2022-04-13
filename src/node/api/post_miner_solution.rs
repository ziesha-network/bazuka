use super::messages::{PostMinerSolutionRequest, PostMinerSolutionResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn post_miner_solution<B: Blockchain>(
    _context: Arc<RwLock<NodeContext<B>>>,
    _req: PostMinerSolutionRequest,
) -> Result<PostMinerSolutionResponse, NodeError> {
    unimplemented!();
}
