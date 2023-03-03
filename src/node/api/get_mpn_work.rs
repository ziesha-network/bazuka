use super::messages::{GetMpnWorkRequest, GetMpnWorkResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_mpn_work<B: Blockchain>(
    _context: Arc<RwLock<NodeContext<B>>>,
    _req: GetMpnWorkRequest,
) -> Result<GetMpnWorkResponse, NodeError> {
    Ok(GetMpnWorkResponse {})
}
