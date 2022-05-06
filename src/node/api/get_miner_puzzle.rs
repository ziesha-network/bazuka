use super::messages::{GetMinerPuzzleRequest, Puzzle};
use super::{Network, NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_miner_puzzle<B: Blockchain, N: Network>(
    context: Arc<RwLock<NodeContext<N, B>>>,
    _req: GetMinerPuzzleRequest,
) -> Result<Puzzle, NodeError> {
    let mut context = context.write().await;
    if let Some((_, puzzle)) = context.miner.as_ref().unwrap().block_puzzle.as_ref() {
        Ok(puzzle.clone())
    } else {
        let wallet = context.wallet.clone().ok_or(NodeError::NoWalletError)?;
        let (blk, puzzle) = context.get_puzzle(wallet)?;
        context.miner.as_mut().unwrap().block_puzzle = Some((blk, puzzle.clone()));
        Ok(puzzle)
    }
}
