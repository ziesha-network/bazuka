mod log_info;

mod discover_peers;
mod refresh;
mod sync_blocks;
mod sync_clock;
mod sync_mempool;
mod sync_peers;
mod sync_state;

use super::{http, Limit, NodeContext, NodeError, Peer, PeerAddress};
use crate::blockchain::Blockchain;
use crate::client::messages::*;
use crate::utils;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockWriteGuard};

pub async fn heartbeat<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    if let Err(e) = log_info::log_info(&context).await {
        log::debug!("info log failed {}", e);
    }
    if let Err(e) = refresh::refresh(&context).await {
        log::error!("refresh failed {}", e);
    }

    // these sets of tasks can be run concurrently as they mix between claiming read/write locks
    let (sync_peers, disc_peers, sync_clock) = tokio::join!(
        sync_peers::sync_peers(&context),
        discover_peers::discover_peers(&context),
        sync_clock::sync_clock(&context),
    );
    if let Err(e) = sync_peers {
        log::error!("peer sync failed {}", e);
    }
    if let Err(e) = disc_peers {
        log::error!("peer discovery failed {}", e);
    }
    if let Err(e) = sync_clock {
        log::error!("clock sync failed {}", e);
    }
    if let Err(e) = sync_blocks::sync_blocks(&context).await {
        log::error!("block sync failed {}", e);
    }
    if let Err(e) = sync_mempool::sync_mempool(&context).await {
        log::error!("mempool sync failed {}", e)
    }

    if let Err(e) = sync_state::sync_state(&context).await {
        log::error!("sync state failed {}", e);
    }
    Ok(())
}

pub async fn heartbeater<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    let mut ticker = tokio::time::interval(context.read().await.opts.heartbeat_interval);
    loop {
        if context.read().await.shutdown {
            break;
        }
        ticker.tick().await;
        if let Err(e) = heartbeat(Arc::clone(&context)).await {
            log::error!("heartbeat failed {}", e);
        }
    }

    Ok(())
}

fn punish_non_responding<B: Blockchain, R: Clone, E>(
    ctx: &mut RwLockWriteGuard<'_, NodeContext<B>>,
    resps: &[(Peer, Result<R, E>)],
) -> Vec<(PeerAddress, R)> {
    resps
        .iter()
        .filter_map(|(peer, resp)| {
            if let Ok(resp) = resp {
                Some((peer.address, resp.clone()))
            } else {
                ctx.punish_unresponsive(peer.address);
                None
            }
        })
        .collect()
}
