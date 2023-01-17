use super::*;
use crate::blockchain::TransactionStats;
use crate::common::*;

pub async fn sync_mempool<B: Blockchain>(
    context: &Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    let ctx = context.read().await;

    let net = ctx.outgoing.clone();
    let opts = ctx.opts.clone();

    let peer_addresses = ctx.peer_manager.get_peers(opts.num_peers);
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
        let now = ctx.local_timestamp();
        let resps = punish_non_responding(&mut ctx, &peer_responses)
            .into_iter()
            .map(|(_, r)| (r.chain_sourced, r.mpn_sourced))
            .collect::<Vec<_>>();
        for (chained_source_txs, mpn_sourced_txs) in resps {
            for tx in chained_source_txs {
                ctx.mempool
                    .chain_sourced
                    .entry(tx)
                    .or_insert(TransactionStats { first_seen: now });
            }
            for tx in mpn_sourced_txs {
                ctx.mempool
                    .mpn_sourced
                    .entry(tx)
                    .or_insert(TransactionStats { first_seen: now });
            }
        }
    }

    Ok(())
}
