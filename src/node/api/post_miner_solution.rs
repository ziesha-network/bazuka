use super::messages::{PostMinerSolutionRequest, PostMinerSolutionResponse};
use super::{Network, NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn post_miner_solution<B: Blockchain, N: Network>(
    context: Arc<RwLock<NodeContext<N, B>>>,
    req: PostMinerSolutionRequest,
) -> Result<PostMinerSolutionResponse, NodeError> {
    let mut context = context.write().await;

    let mut nonce_bytes = [0u8; 8];
    nonce_bytes.copy_from_slice(&hex::decode(req.nonce).unwrap());
    let (mut block, _) = context
        .miner
        .as_ref()
        .ok_or(NodeError::NoMinerError)?
        .block_puzzle
        .as_ref()
        .ok_or(NodeError::NoCurrentlyMiningBlockError)?
        .clone();
    block.header.proof_of_work.nonce = u64::from_le_bytes(nonce_bytes);
    if context
        .blockchain
        .extend(block.header.number as usize, &[block])
        .is_ok()
    {
        context.miner.as_mut().unwrap().block_puzzle = None;
    }
    Ok(PostMinerSolutionResponse {})
}
