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
        nonce: context.blockchain.get_nonce(req.address.parse()?)?,
        mpn_deposit_nonce: context.blockchain.get_deposit_nonce(
            req.address.parse()?,
            context.blockchain.config().mpn_config.mpn_contract_id,
        )?,
    })
}

#[cfg(test)]
use super::tests::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Address;
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
                nonce: 0,
                mpn_deposit_nonce: 0
            }
        );
        assert_eq!(
            resp_treasury,
            GetAccountResponse {
                nonce: 0,
                mpn_deposit_nonce: 0
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
