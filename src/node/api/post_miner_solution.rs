use super::messages::{
    PostBlockRequest, PostBlockResponse, PostMinerSolutionRequest, PostMinerSolutionResponse,
};
use super::{http, Limit, NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::common::*;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn post_miner_solution<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: PostMinerSolutionRequest,
) -> Result<PostMinerSolutionResponse, NodeError> {
    let mut context = context.write().await;
    let net = context.outgoing.clone();

    let mut nonce_bytes = [0u8; 8];
    nonce_bytes.copy_from_slice(&hex::decode(req.nonce).unwrap());
    if let Some((mut draft, _)) = context.miner_puzzle.clone() {
        draft.block.header.proof_of_work.nonce = u64::from_le_bytes(nonce_bytes);
        if context
            .blockchain
            .extend(draft.block.header.number, &[draft.block.clone()])
            .is_ok()
        {
            context.on_update()?;
            let _ = context.blockchain.update_states(&draft.patch.clone());

            let peer_addresses = context.peer_manager.get_peers(context.opts.num_peers);
            http::group_request(&peer_addresses, |peer| {
                net.bincode_post::<PostBlockRequest, PostBlockResponse>(
                    format!("http://{}/bincode/blocks", peer.address),
                    PostBlockRequest {
                        block: draft.block.clone(),
                        patch: draft.patch.clone(),
                    },
                    Limit::default().size(1 * KB).time(3 * SECOND),
                )
            })
            .await;

            context.miner_puzzle = None;
        }
    } else {
        log::info!("No block is currently being mined!");
    }
    Ok(PostMinerSolutionResponse {})
}
