use super::messages::{GetMinerPuzzleRequest, GetMinerPuzzleResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_miner_puzzle<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    _req: GetMinerPuzzleRequest,
) -> Result<GetMinerPuzzleResponse, NodeError> {
    let mut context = context.write().await;
    if let Some((_, puzzle)) = context.miner_puzzle.as_ref() {
        Ok(GetMinerPuzzleResponse {
            puzzle: Some(puzzle.clone()),
        })
    } else {
        let wallet = context.wallet.clone();
        context.miner_puzzle = context.get_puzzle(wallet)?;
        Ok(GetMinerPuzzleResponse {
            puzzle: context.miner_puzzle.as_ref().map(|(_, pzl)| pzl.clone()),
        })
    }
}
