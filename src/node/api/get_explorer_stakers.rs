use super::messages::{GetExplorerStakersRequest, GetExplorerStakersResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::node::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_explorer_stakers<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    _req: GetExplorerStakersRequest,
) -> Result<GetExplorerStakersResponse, NodeError> {
    let context = context.read().await;
    let current = context.blockchain.get_stakers()?;
    Ok(GetExplorerStakersResponse {
        current: current.iter().map(|b| b.into()).collect(),
    })
}

#[cfg(test)]
use super::tests::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_explorer_stakers() {
        let expected = "GetExplorerStakersResponse { current: [ExplorerStaker { pub_key: \"ed062ef0fde01e8544dad7e8c6541c04122e1d70e6b5e89f128a0cfbff617f7cb3\", stake: 25 }, ExplorerStaker { pub_key: \"ed2a141799ef60019f6254aaffc57ffd9b693b8ea4156a4c08965e42cfec26dc6b\", stake: 25 }, ExplorerStaker { pub_key: \"ed6e95016e0a3d299a6e761921da491da1f27189e8a340dfae212daa629853357b\", stake: 25 }] }";
        let ctx = test_context();
        let resp = get_explorer_stakers(ctx.clone(), GetExplorerStakersRequest {})
            .await
            .unwrap();
        assert_eq!(format!("{:?}", resp), expected);
    }
}
