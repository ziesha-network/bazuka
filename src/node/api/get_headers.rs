use super::messages::{GetHeadersRequest, GetHeadersResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::db::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_headers<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: GetHeadersRequest,
) -> Result<GetHeadersResponse, NodeError> {
    let context = context.read().await;
    let count = std::cmp::min(context.opts.max_blocks_fetch, req.count);
    let headers = context.blockchain.get_headers(req.since, count)?;
    Ok(GetHeadersResponse { headers })
}

#[cfg(test)]
use super::tests::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_headers() {
        let ctx = test_context();
        let resp = get_headers(
            ctx.clone(),
            GetHeadersRequest {
                since: 10,
                count: 10,
            },
        )
        .await
        .unwrap();
        let block_indices = resp.headers.iter().map(|b| b.number).collect::<Vec<_>>();
        assert_eq!(block_indices, vec![10, 11, 12, 13, 14, 15, 16, 17, 18, 19]);
    }

    #[tokio::test]
    async fn test_get_headers_max() {
        let ctx = test_context();
        let resp = get_headers(
            ctx.clone(),
            GetHeadersRequest {
                since: 10,
                count: 10000,
            },
        )
        .await
        .unwrap();
        let block_indices = resp.headers.iter().map(|b| b.number).collect::<Vec<_>>();
        assert_eq!(
            block_indices,
            vec![10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25]
        );
    }

    #[tokio::test]
    async fn test_get_headers_overflow() {
        let ctx = test_context();
        let resp = get_headers(
            ctx.clone(),
            GetHeadersRequest {
                since: 99,
                count: 10000,
            },
        )
        .await
        .unwrap();
        let block_indices = resp.headers.iter().map(|b| b.number).collect::<Vec<_>>();
        assert_eq!(block_indices, vec![99, 100]);
    }

    #[tokio::test]
    async fn test_get_headers_non_existing() {
        let ctx = test_context();
        let resp = get_headers(
            ctx.clone(),
            GetHeadersRequest {
                since: 200,
                count: 10,
            },
        )
        .await
        .unwrap();
        let block_indices = resp.headers.iter().map(|b| b.number).collect::<Vec<_>>();
        assert!(block_indices.is_empty());
    }
}
