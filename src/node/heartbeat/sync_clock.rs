use super::*;
use crate::common::*;

pub async fn sync_clock<B: Blockchain>(
    context: &Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    let ctx = context.read().await;

    let handshake_req = match ctx.get_info()? {
        Some(peer) => HandshakeRequest::Node(peer),
        None => HandshakeRequest::Client,
    };

    let opts = ctx.opts.clone();

    let net = ctx.outgoing.clone();

    let peer_addresses = ctx.random_peers(&mut rand::thread_rng(), opts.num_peers);
    drop(ctx);

    log::info!("Syncing clocks with: {:?}", peer_addresses);
    let peer_responses: Vec<(Peer, Result<HandshakeResponse, NodeError>)> =
        http::group_request(&peer_addresses, |peer| {
            net.json_post::<HandshakeRequest, HandshakeResponse>(
                format!("{}/peers", peer.address),
                handshake_req.clone(),
                Limit::default().size(1 * KB).time(3 * SECOND),
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
