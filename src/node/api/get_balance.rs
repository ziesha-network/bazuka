use super::messages::{GetBalanceRequest, GetBalanceResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::core::TokenId;
use crate::db::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_balance<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
    req: GetBalanceRequest,
) -> Result<GetBalanceResponse, NodeError> {
    let context = context.read().await;
    let token_id: TokenId = req.token_id.parse()?;
    let tkn = context
        .blockchain
        .get_token(token_id)?
        .ok_or(crate::blockchain::BlockchainError::TokenNotFound)?;
    Ok(GetBalanceResponse {
        balance: context
            .blockchain
            .get_balance(req.address.parse()?, token_id)?,
        name: tkn.name,
        symbol: tkn.symbol,
    })
}

#[cfg(test)]
use super::tests::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Amount;
    use crate::node::TxBuilder;

    #[tokio::test]
    async fn test_get_balance() {
        let ctx = test_context();
        let abc_addr = TxBuilder::new(&Vec::from("ABC")).get_address();
        let resp = get_balance(
            ctx.clone(),
            GetBalanceRequest {
                token_id: "Ziesha".into(),
                address: abc_addr.to_string(),
            },
        )
        .await
        .unwrap();
        assert_eq!(
            resp,
            GetBalanceResponse {
                balance: Amount::new(10000),
                name: "Ziesha".into(),
                symbol: "ZSH".into(),
            }
        );
    }

    #[tokio::test]
    async fn test_get_balance_invalid_token() {
        let ctx = test_context();
        let abc_addr = TxBuilder::new(&Vec::from("ABC")).get_address();
        let resp = get_balance(
            ctx.clone(),
            GetBalanceRequest {
                token_id: "abcd".into(),
                address: abc_addr.to_string(),
            },
        )
        .await;
        assert!(matches!(
            resp,
            Err(NodeError::TokenIdParseError(
                crate::core::ParseTokenIdError::Invalid
            ))
        ));
    }

    #[tokio::test]
    async fn test_get_balance_invalid_address() {
        let ctx = test_context();
        let resp = get_balance(
            ctx.clone(),
            GetBalanceRequest {
                token_id: "Ziesha".into(),
                address: "abcd".into(),
            },
        )
        .await;
        assert!(matches!(
            resp,
            Err(NodeError::AccountParseAddressError(
                crate::core::ParseAddressError::Invalid
            ))
        ));
    }

    #[tokio::test]
    async fn test_get_balance_non_existing_token() {
        let ctx = test_context();
        let abc_addr = TxBuilder::new(&Vec::from("ABC")).get_address();
        let resp = get_balance(
            ctx.clone(),
            GetBalanceRequest {
                token_id: "0x0000000000000000000000000000000000000000000000000000000000000000"
                    .into(),
                address: abc_addr.to_string(),
            },
        )
        .await;
        assert!(matches!(
            resp,
            Err(NodeError::BlockchainError(
                crate::blockchain::BlockchainError::TokenNotFound
            ))
        ));
    }
}
