use super::messages::{GetExplorerMempoolRequest, GetExplorerMempoolResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::node::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_explorer_mempool<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
    _req: GetExplorerMempoolRequest,
) -> Result<GetExplorerMempoolResponse, NodeError> {
    let context = context.read().await;
    let chain_sourced = context
        .mempool
        .chain_sourced()
        .map(|(tx, stats)| (tx.into(), stats.clone()))
        .collect::<Vec<_>>();
    let mpn_sourced = context
        .mempool
        .mpn_sourced()
        .map(|(tx, stats)| (tx.into(), stats.clone()))
        .collect::<Vec<_>>();
    Ok(GetExplorerMempoolResponse {
        chain_sourced,
        mpn_sourced,
    })
}
