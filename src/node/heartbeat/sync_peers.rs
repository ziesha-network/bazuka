use super::*;

pub async fn sync_peers<B: Blockchain>(
    context: &Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    let ctx = context.read().await;

    let net = ctx.outgoing.clone();

    let peer_addresses = ctx
        .random_peers(&mut rand::thread_rng(), NUM_PEERS)
        .into_iter()
        .map(|p| p.address)
        .collect::<Vec<PeerAddress>>();
    drop(ctx);

    let peer_responses: Vec<(PeerAddress, Result<GetPeersResponse, NodeError>)> =
        http::group_request(&peer_addresses, |peer| {
            net.json_get::<GetPeersRequest, GetPeersResponse>(
                format!("{}/peers", peer),
                GetPeersRequest {},
                Limit::new().size(1024 * 1024).time(1000),
            )
        })
        .await;

    {
        let mut ctx = context.write().await;
        let resps = punish_non_responding(&mut ctx, &peer_responses)
            .into_iter()
            .map(|(_, r)| r.peers)
            .collect::<Vec<_>>();
        for peers in resps {
            for p in peers {
                ctx.peers.entry(p.address).or_insert(Peer {
                    address: p.address,
                    info: None,
                    punished_until: 0,
                });
            }
        }
    }

    Ok(())
}
