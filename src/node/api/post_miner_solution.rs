use super::messages::{PostMinerSolutionRequest, PostMinerSolutionResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn post_miner_solution<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: PostMinerSolutionRequest,
) -> Result<PostMinerSolutionResponse, NodeError> {
    let mut context = context.write().await;

    let mut nonce_bytes = [0u8; 8];
    nonce_bytes.copy_from_slice(&hex::decode(req.nonce).unwrap());
    let (mut draft, _) = context
        .miner_puzzle
        .as_ref()
        .ok_or(NodeError::NoCurrentlyMiningBlockError)?
        .clone();
    draft.block.header.proof_of_work.nonce = u64::from_le_bytes(nonce_bytes);
    if context
        .blockchain
        .extend(draft.block.header.number, &[draft.block])
        .is_ok()
    {
        let _ = context.blockchain.update_states(&draft.patch);
        context.miner_puzzle = None;
    }
    Ok(PostMinerSolutionResponse {})
}
