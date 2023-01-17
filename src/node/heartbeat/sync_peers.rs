use super::*;
use crate::common::*;
use crate::utils::local_timestamp;
use rand::prelude::IteratorRandom;

pub async fn sync_peers<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    let ctx = context.read().await;

    let net = ctx.outgoing.clone();
    let opts = ctx.opts.clone();

    let peer_addresses = ctx.peer_manager.get_peers();
    drop(ctx);

    log::info!("Syncing peers...");
    let peer_responses: Vec<(Peer, Result<GetPeersResponse, NodeError>)> =
        http::group_request(&peer_addresses, |peer| {
            net.json_get::<GetPeersRequest, GetPeersResponse>(
                format!("http://{}/peers", peer.address),
                GetPeersRequest {},
                Limit::default().size(1 * MB).time(3 * SECOND),
            )
        })
        .await;

    {
        let mut ctx = context.write().await;

        let resps = punish_non_responding(&mut ctx, &peer_responses)
            .into_iter()
            .map(|(_, r)| r.peers)
            .collect::<Vec<_>>();

        let now = local_timestamp();

        for peers in resps {
            for p in peers
                .into_iter()
                .choose_multiple(&mut rand::thread_rng(), ctx.opts.num_peers)
                .into_iter()
            {
                ctx.peer_manager.add_candidate(now, p);
            }
        }

        ctx.peer_manager.select_peers(opts.num_peers);
    }

    Ok(())
}
