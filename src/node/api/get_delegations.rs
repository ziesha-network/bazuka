use super::messages::{GetDelegationsRequest, GetDelegationsResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::db::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_delegations<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
    req: GetDelegationsRequest,
) -> Result<GetDelegationsResponse, NodeError> {
    let context = context.read().await;
    Ok(GetDelegationsResponse {
        delegatees: context
            .blockchain
            .get_delegatees(req.address.parse()?, Some(req.top))?
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
        delegators: context
            .blockchain
            .get_delegators(req.address.parse()?, Some(req.top))?
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    })
}

#[cfg(test)]
use super::tests::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Amount;
    use crate::wallet::TxBuilder;

    #[tokio::test]
    async fn test_get_delegations_delegators() {
        let validator = TxBuilder::new(&Vec::from("VALIDATOR"));
        let delegator = TxBuilder::new(&Vec::from("DELEGATOR"));
        let ctx = test_context();
        let resp = get_delegations(
            ctx.clone(),
            GetDelegationsRequest {
                address: validator.get_address().to_string(),
                top: 100,
            },
        )
        .await
        .unwrap();
        assert_eq!(resp.delegatees.len(), 0);
        assert_eq!(resp.delegators.len(), 1);
        assert_eq!(
            resp.delegators
                .get(&delegator.get_address().to_string())
                .cloned(),
            Some(Amount::new(25))
        );
    }

    #[tokio::test]
    async fn test_get_delegations_delegatees() {
        let validator1 = TxBuilder::new(&Vec::from("VALIDATOR"));
        let validator2 = TxBuilder::new(&Vec::from("VALIDATOR2"));
        let validator3 = TxBuilder::new(&Vec::from("VALIDATOR3"));
        let delegator = TxBuilder::new(&Vec::from("DELEGATOR"));
        let ctx = test_context();
        let resp = get_delegations(
            ctx.clone(),
            GetDelegationsRequest {
                address: delegator.get_address().to_string(),
                top: 100,
            },
        )
        .await
        .unwrap();
        assert_eq!(resp.delegatees.len(), 3);
        assert_eq!(resp.delegators.len(), 0);
        for val in [validator1, validator2, validator3] {
            assert_eq!(
                resp.delegatees.get(&val.get_address().to_string()).cloned(),
                Some(Amount::new(25))
            );
        }
    }
}
