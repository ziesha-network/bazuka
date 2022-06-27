use super::*;

pub async fn sync_peers<B: Blockchain>(
    context: &Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    let ctx = context.read().await;

    let net = ctx.outgoing.clone();
    let opts = ctx.opts.clone();

    let peer_addresses = ctx.random_peers(&mut rand::thread_rng(), opts.num_peers);
    drop(ctx);

    let peer_responses: Vec<(Peer, Result<GetPeersResponse, NodeError>)> =
        http::group_request(&peer_addresses, |peer| {
            net.json_get::<GetPeersRequest, GetPeersResponse>(
                format!("{}/peers", peer.address),
                GetPeersRequest {},
                Limit::default().size(1024 * 1024).time(1000),
            )
        })
        .await;

    {
        let mut ctx = context.write().await;
        let resps = punish_non_responding(&mut ctx, &peer_responses, opts.no_response_punish)
            .into_iter()
            .map(|(_, r)| r.peers)
            .collect::<Vec<_>>();
        for peers in resps {
            for p in peers {
                ctx.peers.entry(p.address).or_insert(Peer {
                    pub_key: None,
                    address: p.address,
                    info: None,
                    punished_until: 0,
                });
            }
        }
    }

    Ok(())
}
