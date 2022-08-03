use super::messages::{GetMpnAccountRequest, GetMpnAccountResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::{Blockchain, BlockchainError};
use crate::config::blockchain::MPN_CONTRACT_ID;
use crate::crypto::jubjub;
use crate::zk::{MpnAccount, ZkDataLocator, ZkScalar};
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_mpn_account<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: GetMpnAccountRequest,
) -> Result<GetMpnAccountResponse, NodeError> {
    let context = context.read().await;
    let cells = (0..4)
        .map(|i| {
            context
                .blockchain
                .read_state(*MPN_CONTRACT_ID, ZkDataLocator(vec![req.index, i as u32]))
        })
        .collect::<Result<Vec<ZkScalar>, BlockchainError>>()?;

    Ok(GetMpnAccountResponse {
        account: MpnAccount {
            nonce: 0, // TODO
            address: jubjub::PointAffine(cells[1], cells[2]),
            balance: 0, // TODO
        },
    })
}
