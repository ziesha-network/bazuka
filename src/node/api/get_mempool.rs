use super::messages::{GetMempoolRequest, GetMempoolResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::core::{GeneralTransaction, TransactionData};
use crate::db::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_mempool<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
    req: GetMempoolRequest,
) -> Result<GetMempoolResponse, NodeError> {
    let context = context.read().await;
    let mpn_contract_id = context.blockchain.config().mpn_config.mpn_contract_id;
    Ok(GetMempoolResponse {
        mempool: context
            .mempool
            .all()
            .filter_map(|(tx, _)| {
                // Do not share MPN txs with others! It's a competetion :)
                if let GeneralTransaction::TransactionAndDelta(tx) = &tx {
                    if let TransactionData::UpdateContract { contract_id, .. } = &tx.tx.data {
                        if contract_id == &mpn_contract_id {
                            return None;
                        }
                    }
                }
                if let Some(filter) = req.filter.clone() {
                    if tx.sender() != filter {
                        return None;
                    }
                    // TODO: Also apply filter on dst!
                }
                Some(tx.clone())
            })
            .collect(),
    })
}
