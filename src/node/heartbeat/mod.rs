mod log_info;
#[cfg(feature = "pow")]
mod send_mining_puzzle;
mod sync_blocks;
mod sync_clock;
mod sync_peers;

use super::api::messages::*;
use super::{http, NodeContext, NodeError, Peer, PeerAddress};
use crate::blockchain::Blockchain;
use crate::config::punish;
use crate::utils;
use std::sync::Arc;
use tokio::join;
use tokio::sync::{RwLock, RwLockWriteGuard};
use tokio::time::{sleep, Duration};

const NUM_PEERS: usize = 8;

pub async fn heartbeat<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    log_info::log_info(&context).await?;
    sync_clock::sync_clock(&context).await?;
    sync_peers::sync_peers(&context).await?;
    sync_blocks::sync_blocks(&context).await?;
    #[cfg(feature = "pow")]
    send_mining_puzzle::send_mining_puzzle(&context).await?;
    Ok(())
}

pub async fn heartbeater<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    loop {
        if context.read().await.shutdown {
            break;
        }
        let heartbeat_future = heartbeat(Arc::clone(&context));
        let sleep_future = sleep(Duration::from_millis(1000));
        let (res, _) = join!(heartbeat_future, sleep_future);
        if let Err(e) = res {
            log::error!("Error happened: {}", e);
        }
    }

    Ok(())
}

fn punish_non_responding<B: Blockchain, R: Clone, E>(
    ctx: &mut RwLockWriteGuard<'_, NodeContext<B>>,
    resps: &[(PeerAddress, Result<R, E>)],
) -> Vec<(PeerAddress, R)> {
    resps
        .iter()
        .filter_map(|(peer, resp)| {
            if let Ok(resp) = resp {
                Some((*peer, resp.clone()))
            } else {
                ctx.punish(*peer, punish::NO_RESPONSE_PUNISH);
                None
            }
        })
        .collect()
}
