use super::messages::{GetBlocksRequest, GetBlocksResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_blocks<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    _req: GetBlocksRequest,
) -> Result<GetBlocksResponse, NodeError> {
    unimplemented!();
}
