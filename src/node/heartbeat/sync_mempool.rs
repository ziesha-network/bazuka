use super::*;
use crate::blockchain::TransactionStats;
use crate::common::*;

pub async fn sync_mempool<B: Blockchain>(
    context: &Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    let ctx = context.read().await;

    let net = ctx.outgoing.clone();
    let opts = ctx.opts.clone();

    let peer_addresses = ctx.random_peers(&mut rand::thread_rng(), opts.num_peers);
    drop(ctx);

    let peer_responses: Vec<(Peer, Result<GetMempoolResponse, NodeError>)> =
        http::group_request(&peer_addresses, |peer| {
            net.bincode_get::<GetMempoolRequest, GetMempoolResponse>(
                format!("{}/bincode/mempool", peer.address),
                GetMempoolRequest {},
                Limit::default().size(10 * MB).time(10 * SECOND),
            )
        })
        .await;

    {
        let mut ctx = context.write().await;
        let now = ctx.network_timestamp();
        let resps = punish_non_responding(&mut ctx, &peer_responses, opts.no_response_punish)
            .into_iter()
            .map(|(_, r)| (r.tx, r.tx_zk, r.zk))
            .collect::<Vec<_>>();
        for (tx_s, tx_zk_s, zk_s) in resps {
            for tx in tx_s {
                ctx.mempool
                    .entry(tx)
                    .or_insert(TransactionStats { first_seen: now });
            }
            for tx in tx_zk_s {
                ctx.contract_payment_mempool
                    .entry(tx)
                    .or_insert(TransactionStats { first_seen: now });
            }
            for tx in zk_s {
                ctx.zero_mempool
                    .entry(tx)
                    .or_insert(TransactionStats { first_seen: now });
            }
        }
    }

    Ok(())
}
