use super::messages::{GetAccountRequest, GetAccountResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::db::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_account<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
    req: GetAccountRequest,
) -> Result<GetAccountResponse, NodeError> {
    let context = context.read().await;
    Ok(GetAccountResponse {
        account: context.blockchain.get_account(req.address.parse()?)?,
    })
}

#[cfg(test)]
use super::tests::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Account, Address};
    use crate::node::TxBuilder;

    #[tokio::test]
    async fn test_get_account() {
        let ctx = test_context();
        let abc_addr = TxBuilder::new(&Vec::from("ABC")).get_address();
        let treasury_addr: Address = Default::default();
        let resp = get_account(
            ctx.clone(),
            GetAccountRequest {
                address: abc_addr.to_string(),
            },
        )
        .await
        .unwrap();
        let resp_treasury = get_account(
            ctx.clone(),
            GetAccountRequest {
                address: treasury_addr.to_string(),
            },
        )
        .await
        .unwrap();
        assert_eq!(
            resp,
            GetAccountResponse {
                account: Account { nonce: 0 }
            }
        );
        assert_eq!(
            resp_treasury,
            GetAccountResponse {
                account: Account { nonce: 4 }
            }
        );
        let resp_invalid = get_account(
            ctx.clone(),
            GetAccountRequest {
                address: "invalid".into(),
            },
        )
        .await;
        assert!(matches!(
            resp_invalid,
            Err(NodeError::AccountParseAddressError(
                crate::core::ParseAddressError::Invalid
            ))
        ));
    }
}
