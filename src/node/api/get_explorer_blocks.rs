use super::messages::{GetExplorerBlocksRequest, GetExplorerBlocksResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::db::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_explorer_blocks<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: GetExplorerBlocksRequest,
) -> Result<GetExplorerBlocksResponse, NodeError> {
    let context = context.read().await;
    let count = std::cmp::min(context.opts.max_blocks_fetch, req.count);
    let blocks = context.blockchain.get_blocks(req.since, count)?;
    Ok(GetExplorerBlocksResponse {
        blocks: blocks.iter().map(|b| b.into()).collect(),
    })
}

#[cfg(test)]
use super::tests::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_explorer_blocks_format() {
        let expected = "[ExplorerBlock { header: ExplorerHeader { parent_hash: \"0000000000000000000000000000000000000000000000000000000000000000\", number: 0, block_root: \"0000000000000000000000000000000000000000000000000000000000000000\", proof_of_stake: ExplorerProofOfStake { timestamp: 0, validator: \"ed0000000000000000000000000000000000000000000000000000000000000000\" } }, body: [ExplorerTransaction { memo: \"Happy Birthday Ziesha!\", src: None, nonce: 1, data: CreateToken { token: ExplorerToken { name: \"Ziesha\", symbol: \"ZSH\", supply: 2000000000000000000, minter: None } }, fee: ExplorerMoney { amount: 0, token_id: \"Ziesha\" }, sig: \"\" }, ExplorerTransaction { memo: \"A Payment-Network to rule them all!\", src: None, nonce: 2, data: CreateContract { contract: ExplorerContract { initial_state: ExplorerCompressedState { state: ZkCompressedState { state_hash: ZkScalar(0x501a18871f186db1437e77e2c33acfa81405608cc60806399347215dbe98f714), state_size: 0 } }, state_model: ExplorerStateModel { state_model: List { log4_size: 30, item_type: Struct { field_types: [Scalar, Scalar, Scalar, Scalar, List { log4_size: 1, item_type: Struct { field_types: [Scalar, Scalar] } }] } } }, deposit_functions: [ExplorerMultiInputVerifierKey { verifier_key: ExplorerVerifierKey { vk: Dummy }, log4_payment_capacity: 1 }], withdraw_functions: [ExplorerMultiInputVerifierKey { verifier_key: ExplorerVerifierKey { vk: Dummy }, log4_payment_capacity: 1 }], functions: [ExplorerSingleInputVerifierKey { verifier_key: ExplorerVerifierKey { vk: Dummy } }] } }, fee: ExplorerMoney { amount: 0, token_id: \"Ziesha\" }, sig: \"\" }, ExplorerTransaction { memo: \"Dummy tx\", src: None, nonce: 3, data: RegularSend { entries: [(\"ed8c19c6a4cf1460e961f7bae8eea54d437b9edac27cbeb09be32ae367adf9098a\", ExplorerMoney { amount: 10000, token_id: \"Ziesha\" })] }, fee: ExplorerMoney { amount: 0, token_id: \"Ziesha\" }, sig: \"\" }, ExplorerTransaction { memo: \"\", src: None, nonce: 4, data: RegularSend { entries: [(\"ed379d481f1e818af8c5c10f7488f5765c2a87bd10c01699b2309dbc9ab81efe21\", ExplorerMoney { amount: 100, token_id: \"Ziesha\" })] }, fee: ExplorerMoney { amount: 0, token_id: \"Ziesha\" }, sig: \"\" }, ExplorerTransaction { memo: \"Test validator\", src: Some(\"ed062ef0fde01e8544dad7e8c6541c04122e1d70e6b5e89f128a0cfbff617f7cb3\"), nonce: 1, data: UpdateStaker { vrf_pub_key: \"0c8b08e1af55ac2907f2b18d3bfb11ffa9feb21b8a782ce236bbefd769d09532\", commision: 12 }, fee: ExplorerMoney { amount: 0, token_id: \"Ziesha\" }, sig: \"\" }, ExplorerTransaction { memo: \"\", src: Some(\"ed379d481f1e818af8c5c10f7488f5765c2a87bd10c01699b2309dbc9ab81efe21\"), nonce: 1, data: Delegate { to: \"ed062ef0fde01e8544dad7e8c6541c04122e1d70e6b5e89f128a0cfbff617f7cb3\", amount: 25, reverse: false }, fee: ExplorerMoney { amount: 0, token_id: \"Ziesha\" }, sig: \"\" }, ExplorerTransaction { memo: \"Test validator\", src: Some(\"ed6e95016e0a3d299a6e761921da491da1f27189e8a340dfae212daa629853357b\"), nonce: 1, data: UpdateStaker { vrf_pub_key: \"b4d9ae5e4152bc7efc2aac9c17042282e11042d9879df3d98caab368b642f15c\", commision: 12 }, fee: ExplorerMoney { amount: 0, token_id: \"Ziesha\" }, sig: \"\" }, ExplorerTransaction { memo: \"\", src: Some(\"ed379d481f1e818af8c5c10f7488f5765c2a87bd10c01699b2309dbc9ab81efe21\"), nonce: 2, data: Delegate { to: \"ed6e95016e0a3d299a6e761921da491da1f27189e8a340dfae212daa629853357b\", amount: 25, reverse: false }, fee: ExplorerMoney { amount: 0, token_id: \"Ziesha\" }, sig: \"\" }, ExplorerTransaction { memo: \"Test validator\", src: Some(\"ed2a141799ef60019f6254aaffc57ffd9b693b8ea4156a4c08965e42cfec26dc6b\"), nonce: 1, data: UpdateStaker { vrf_pub_key: \"5c85a1ae211a922515629683725a1e244be0061a778f15d80b89b6008546f952\", commision: 12 }, fee: ExplorerMoney { amount: 0, token_id: \"Ziesha\" }, sig: \"\" }, ExplorerTransaction { memo: \"\", src: Some(\"ed379d481f1e818af8c5c10f7488f5765c2a87bd10c01699b2309dbc9ab81efe21\"), nonce: 3, data: Delegate { to: \"ed2a141799ef60019f6254aaffc57ffd9b693b8ea4156a4c08965e42cfec26dc6b\", amount: 25, reverse: false }, fee: ExplorerMoney { amount: 0, token_id: \"Ziesha\" }, sig: \"\" }] }, ExplorerBlock { header: ExplorerHeader { parent_hash: \"2f8d24bb427a4e9d83cf0468862f8a52eee394874d2a749c05813add1d6bd7e5\", number: 1, block_root: \"2f8d24bb427a4e9d83cf0468862f8a52eee394874d2a749c05813add1d6bd7e5\", proof_of_stake: ExplorerProofOfStake { timestamp: 30, validator: \"ed062ef0fde01e8544dad7e8c6541c04122e1d70e6b5e89f128a0cfbff617f7cb3\" } }, body: [] }]";
        let ctx = test_context();
        let blocks =
            get_explorer_blocks(ctx.clone(), GetExplorerBlocksRequest { since: 0, count: 2 })
                .await
                .unwrap()
                .blocks;
        assert_eq!(format!("{:?}", blocks), expected);
    }

    #[tokio::test]
    async fn test_get_explorer_blocks() {
        let ctx = test_context();
        let resp = get_explorer_blocks(
            ctx.clone(),
            GetExplorerBlocksRequest {
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
    async fn test_get_explorer_blocks_max() {
        let ctx = test_context();
        let resp = get_explorer_blocks(
            ctx.clone(),
            GetExplorerBlocksRequest {
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
    async fn test_get_explorer_blocks_max_overflow() {
        let ctx = test_context();
        let resp = get_explorer_blocks(
            ctx.clone(),
            GetExplorerBlocksRequest {
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
    async fn test_get_explorer_blocks_non_existing() {
        let ctx = test_context();
        let resp = get_explorer_blocks(
            ctx.clone(),
            GetExplorerBlocksRequest {
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
