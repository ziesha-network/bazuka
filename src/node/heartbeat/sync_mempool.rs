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
                GetMempoolRequest {},
                Limit::default().size(10 * MB).time(10 * SECOND),
            )
        })
        .await;

    {
        let mut ctx = context.write().await;
        let resps = punish_non_responding(&mut ctx, &peer_responses)
            .into_iter()
            .map(|(_, r)| (r.chain_sourced, r.mpn_sourced))
            .collect::<Vec<_>>();
        for (mut chain_sourced_txs, mut mpn_sourced_txs) in resps {
            chain_sourced_txs.sort_unstable_by_key(|t| t.nonce());
            mpn_sourced_txs.sort_unstable_by_key(|t| t.nonce());
            for tx in chain_sourced_txs
                .into_iter()
                .take(opts.chain_mempool_max_fetch)
            {
                ctx.mempool_add_chain_sourced(false, tx)?;
            }
            for tx in mpn_sourced_txs.into_iter().take(opts.mpn_mempool_max_fetch) {
                ctx.mempool_add_mpn_sourced(false, tx)?;
            }
        }
    }

    Ok(())
}
