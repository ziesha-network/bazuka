use super::*;
use crate::common::*;

pub async fn sync_mempool<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
) -> Result<(), NodeError> {
    let ctx = context.read().await;

    let net = ctx.outgoing.clone();
    let opts = ctx.opts.clone();

    let peer_addresses = ctx.peer_manager.get_peers();
    drop(ctx);

    log::info!("Syncing mempools...");
    let peer_responses: Vec<(Peer, Result<GetMempoolResponse, NodeError>)> =
        http::group_request(&peer_addresses, |peer| {
            net.bincode_get::<GetMempoolRequest, GetMempoolResponse>(
                format!("http://{}/bincode/mempool", peer.address),
                GetMempoolRequest { filter: None },
                Limit::default().size(10 * MB).time(10 * SECOND),
            )
        })
        .await;

    {
        let mut ctx = context.write().await;
        let resps = punish_non_responding(&mut ctx, &peer_responses)
            .into_iter()
            .map(|(_, r)| r.mempool)
            .collect::<Vec<_>>();
        for txs in resps {
            for tx in txs.into_iter().take(opts.mempool_max_fetch) {
                if let Err(e) = ctx.mempool_add_tx(false, tx.clone()) {
                    log::info!("Skipped syncing mempool: {}", e);
                    break;
                }
            }
        }
    }

    Ok(())
}
