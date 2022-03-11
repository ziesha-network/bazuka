use super::messages::{PostBlockRequest, PostBlockResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn post_block<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: PostBlockRequest,
) -> Result<PostBlockResponse, NodeError> {
    unimplemented!();
}
