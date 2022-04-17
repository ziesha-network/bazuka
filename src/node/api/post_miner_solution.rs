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
    if let Some(miner) = context.miner.as_mut() {
        let mut blk = miner.block.take().unwrap();
        let mut nonce_bytes = [0u8; 8];
        nonce_bytes.copy_from_slice(&hex::decode(req.nonce).unwrap());
        blk.header.proof_of_work.nonce = u64::from_le_bytes(nonce_bytes);
        context
            .blockchain
            .extend(blk.header.number as usize, &vec![blk])?;
    }
    Ok(PostMinerSolutionResponse {})
}
