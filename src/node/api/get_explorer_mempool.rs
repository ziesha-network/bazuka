use super::messages::{GetExplorerMempoolRequest, GetExplorerMempoolResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::node::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_explorer_mempool<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    _req: GetExplorerMempoolRequest,
) -> Result<GetExplorerMempoolResponse, NodeError> {
    let context = context.read().await;
    Ok(GetExplorerMempoolResponse {
        mempool: context
            .mempool
            .all()
            .map(|(tx, stats)| (tx.into(), stats.clone()))
            .collect::<Vec<_>>(),
    })
}

#[cfg(test)]
use super::tests::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_explorer_mempool_empty() {
        // TODO: Test cases where mempool is not empty!
        let expected = "GetExplorerMempoolResponse { mempool: [] }";
        let ctx = test_context();
        let resp = get_explorer_mempool(ctx.clone(), GetExplorerMempoolRequest {})
            .await
            .unwrap();
        assert_eq!(format!("{:?}", resp), expected);
    }
}
