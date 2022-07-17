mod log_info;

mod sync_blocks;
mod sync_clock;
mod sync_peers;
mod sync_state;

use super::{http, Limit, NodeContext, NodeError, Peer, PeerAddress};
use crate::blockchain::Blockchain;
use crate::client::messages::*;
use crate::utils;
use std::sync::Arc;
use tokio::join;
use tokio::sync::{RwLock, RwLockWriteGuard};
use tokio::time::sleep;

pub async fn heartbeat<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    log_info::log_info(&context).await?;
    sync_clock::sync_clock(&context).await?;
    sync_peers::sync_peers(&context).await?;
    sync_blocks::sync_blocks(&context).await?;
    sync_state::sync_state(&context).await?;
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
        let sleep_future = sleep(context.read().await.opts.heartbeat_interval);
        let (res, _) = join!(heartbeat_future, sleep_future);
        if let Err(e) = res {
            log::error!("Error happened: {}", e);
        }
    }

    Ok(())
}

fn punish_non_responding<B: Blockchain, R: Clone, E>(
    ctx: &mut RwLockWriteGuard<'_, NodeContext<B>>,
    resps: &[(Peer, Result<R, E>)],
    amount: u32,
) -> Vec<(PeerAddress, R)> {
    resps
        .iter()
        .filter_map(|(peer, resp)| {
            if let Ok(resp) = resp {
                Some((peer.address, resp.clone()))
            } else {
                ctx.punish(peer.address, amount);
                None
            }
        })
        .collect()
}
