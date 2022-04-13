use super::messages::{GetMinerPuzzleRequest, GetMinerPuzzleResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_miner_puzzle<B: Blockchain>(
    _context: Arc<RwLock<NodeContext<B>>>,
    _req: GetMinerPuzzleRequest,
) -> Result<GetMinerPuzzleResponse, NodeError> {
    unimplemented!();
}
