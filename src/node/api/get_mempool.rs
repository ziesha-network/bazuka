use super::messages::{GetMempoolRequest, GetMempoolResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::core::TransactionData;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_mempool<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    _req: GetMempoolRequest,
) -> Result<GetMempoolResponse, NodeError> {
    let mut context = context.write().await;
    context.refresh()?;
    let mpn_contract_id = context.blockchain.config().mpn_contract_id;
    Ok(GetMempoolResponse {
        tx: context
            .mempool
            .tx
            .clone()
            .into_keys()
            .filter(|tx| {
                // Do not share MPN txs with others! It's a competetion :)
                if let TransactionData::UpdateContract { contract_id, .. } = &tx.tx.data {
                    contract_id != &mpn_contract_id
                } else {
                    true
                }
            })
            .collect(),
        tx_zk: context.mempool.tx_zk.clone().into_keys().collect(),
        zk: context.mempool.zk.clone().into_keys().collect(),
    })
}
