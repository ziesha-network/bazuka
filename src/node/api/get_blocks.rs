use super::messages::{GetBlocksRequest, GetBlocksResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::db::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_blocks<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
    req: GetBlocksRequest,
) -> Result<GetBlocksResponse, NodeError> {
    let context = context.read().await;
    let count = std::cmp::min(context.opts.max_blocks_fetch, req.count);
    Ok(GetBlocksResponse {
        blocks: context.blockchain.get_blocks(req.since, count)?,
    })
}

#[cfg(test)]
use super::tests::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_blocks() {
        let ctx = test_context();
        let resp = get_blocks(
            ctx.clone(),
            GetBlocksRequest {
                since: 10,
                count: 10,
            },
        )
        .await
        .unwrap();
        let block_indices = resp
            .blocks
            .iter()
            .map(|b| b.header.number)
            .collect::<Vec<_>>();
        assert_eq!(block_indices, vec![10, 11, 12, 13, 14, 15, 16, 17, 18, 19]);
    }

    #[tokio::test]
    async fn test_get_blocks_max() {
        let ctx = test_context();
        let resp = get_blocks(
            ctx.clone(),
            GetBlocksRequest {
                since: 10,
                count: 10000,
            },
        )
        .await
        .unwrap();
        let block_indices = resp
            .blocks
            .iter()
            .map(|b| b.header.number)
            .collect::<Vec<_>>();
        assert_eq!(
            block_indices,
            vec![10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25]
        );
    }

    #[tokio::test]
    async fn test_get_blocks_overflow() {
        let ctx = test_context();
        let resp = get_blocks(
            ctx.clone(),
            GetBlocksRequest {
                since: 99,
                count: 10000,
            },
        )
        .await
        .unwrap();
        let block_indices = resp
            .blocks
            .iter()
            .map(|b| b.header.number)
            .collect::<Vec<_>>();
        assert_eq!(block_indices, vec![99, 100]);
    }

    #[tokio::test]
    async fn test_get_blocks_non_existing() {
        let ctx = test_context();
        let resp = get_blocks(
            ctx.clone(),
            GetBlocksRequest {
                since: 200,
                count: 10,
            },
        )
        .await
        .unwrap();
        let block_indices = resp
            .blocks
            .iter()
            .map(|b| b.header.number)
            .collect::<Vec<_>>();
        assert!(block_indices.is_empty());
    }
}
