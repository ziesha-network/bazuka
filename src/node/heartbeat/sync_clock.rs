use super::*;

pub async fn sync_clock<B: Blockchain>(
    address: PeerAddress,
    context: &Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    let ctx = context.read().await;

    let net = ctx.outgoing.clone();

    let timestamp = ctx.network_timestamp();
    let info = ctx.get_info()?;
    let peer_addresses = ctx
        .random_peers(&mut rand::thread_rng(), NUM_PEERS)
        .into_iter()
        .map(|p| p.address)
        .collect::<Vec<PeerAddress>>();
    drop(ctx);

    let peer_responses: Vec<(PeerAddress, Result<PostPeerResponse, NodeError>)> =
        http::group_request(&peer_addresses, |peer| {
            net.json_post::<PostPeerRequest, PostPeerResponse>(
                format!("{}/peers", peer),
                PostPeerRequest {
                    address,
                    timestamp,
                    info: info.clone(),
                },
            )
        })
        .await;

    {
        let mut ctx = context.write().await;
        let timestamps = punish_non_responding(&mut ctx, &peer_responses)
            .into_iter()
            .map(|(_, r)| r.timestamp)
            .collect::<Vec<_>>();
        if !timestamps.is_empty() {
            // Set timestamp_offset according to median timestamp of the network
            let median_timestamp = utils::median(&timestamps);
            ctx.timestamp_offset = median_timestamp as i32 - utils::local_timestamp() as i32;
        }
    }

    Ok(())
}
