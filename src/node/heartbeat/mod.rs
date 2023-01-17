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
use std::time::Duration;
use tokio::sync::{RwLock, RwLockWriteGuard};

pub async fn make_loop<
    B: Blockchain,
    Fut: futures::Future,
    F: Fn(&Arc<RwLock<NodeContext<B>>>) -> Fut,
>(
    context: &Arc<RwLock<NodeContext<B>>>,
    func: F,
    interval: Duration,
) {
    loop {
        if context.read().await.shutdown {
            break;
        }
        func(context).await;
        tokio::time::sleep(interval).await;
    }
}

pub async fn heartbeater<B: Blockchain>(ctx: Arc<RwLock<NodeContext<B>>>) -> Result<(), NodeError> {
    let dur = ctx.read().await.opts.heartbeat_interval;
    tokio::join!(
        make_loop(&ctx, |ctx| log_info::log_info(ctx.clone()), dur),
        make_loop(&ctx, |ctx| refresh::refresh(ctx.clone()), dur),
        make_loop(&ctx, |ctx| sync_peers::sync_peers(ctx.clone()), dur),
        make_loop(&ctx, |ctx| discover_peers::discover_peers(ctx.clone()), dur),
        make_loop(&ctx, |ctx| sync_clock::sync_clock(ctx.clone()), dur),
        make_loop(&ctx, |ctx| sync_blocks::sync_blocks(ctx.clone()), dur),
        make_loop(&ctx, |ctx| sync_mempool::sync_mempool(ctx.clone()), dur),
        make_loop(&ctx, |ctx| sync_state::sync_state(ctx.clone()), dur),
    );

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
