use super::messages::{GetMinerPuzzleRequest, Puzzle};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_miner_puzzle<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    _req: GetMinerPuzzleRequest,
) -> Result<Puzzle, NodeError> {
    let mut context = context.write().await;
    if let Some((_, puzzle)) = context.miner_puzzle.as_ref() {
        Ok(puzzle.clone())
    } else {
        let wallet = context.wallet.clone().ok_or(NodeError::NoWalletError)?;
        let (blk, puzzle) = context.get_puzzle(wallet)?;
        context.miner_puzzle = Some((blk, puzzle.clone()));
        Ok(puzzle)
    }
}
