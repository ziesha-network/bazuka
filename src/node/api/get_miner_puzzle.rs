use super::messages::{GetMinerPuzzleRequest, GetMinerPuzzleResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_miner_puzzle<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    _req: GetMinerPuzzleRequest,
) -> Result<GetMinerPuzzleResponse, NodeError> {
    let context = context.read().await;
    let wallet = context.wallet.clone().ok_or(NodeError::NoWalletError)?;
    let txs = context.mempool.keys().cloned().collect();
    let blk = context.blockchain.draft_block(&txs, &wallet)?;
    Ok(GetMinerPuzzleResponse {
        key: hex::encode(b"puzzle key"),
        blob: hex::encode(bincode::serialize(&blk.header).unwrap()),
        offset: 112,
        size: 4,
        target: blk.header.proof_of_work.target,
    })
}
