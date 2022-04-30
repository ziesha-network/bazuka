mod log_info;
#[cfg(feature = "pow")]
mod send_mining_puzzle;
mod sync_blocks;
mod sync_clock;

use super::api::messages::*;
use super::{http, NodeContext, NodeError, PeerAddress};
use crate::blockchain::Blockchain;
use crate::config::punish;
use crate::utils;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockWriteGuard};
use tokio::time::{sleep, Duration};

const NUM_PEERS: usize = 8;

pub async fn heartbeat<B: Blockchain>(
    address: PeerAddress,
    context: Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    log_info::log_info(&context).await?;
    sync_clock::sync_clock(address, &context).await?;
    sync_blocks::sync_blocks(&context).await?;
    #[cfg(feature = "pow")]
    send_mining_puzzle::send_mining_puzzle(&context).await?;
    Ok(())
}

pub async fn heartbeater<B: Blockchain>(
    address: PeerAddress,
    context: Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    loop {
        if let Err(e) = heartbeat(address.clone(), Arc::clone(&context)).await {
            println!("Error happened: {}", e);
        }
        sleep(Duration::from_millis(1000)).await;
    }
}

async fn punish_non_responding<B: Blockchain, R: Clone, E>(
    ctx: &mut RwLockWriteGuard<'_, NodeContext<B>>,
    resps: &Vec<(PeerAddress, Result<R, E>)>,
) -> Vec<(PeerAddress, R)> {
    resps
        .iter()
        .filter_map(|(peer, resp)| {
            if let Ok(resp) = resp {
                ctx.punish(peer.clone(), punish::NO_RESPONSE_PUNISH);
                Some((peer.clone(), resp.clone()))
            } else {
                None
            }
        })
        .collect()
}
