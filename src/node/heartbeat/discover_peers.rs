use super::*;
use crate::common::*;
use std::time::{Duration, Instant};

pub async fn discover_peers<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
) -> Result<(), NodeError> {
    let ctx = context.read().await;

    let handshake_req = match ctx.get_info()? {
        Some(peer) => HandshakeRequest::Node(peer.address),
        None => HandshakeRequest::Client,
    };

    let opts = ctx.opts.clone();

    let net = ctx.outgoing.clone();

    let peer_addresses = ctx.peer_manager.random_candidates(opts.num_peers);
    drop(ctx);

    let peer_responses: Vec<(
        PeerAddress,
        Result<(HandshakeResponse, Duration), NodeError>,
    )> = http::group_request(&peer_addresses, move |peer| {
        let handshake_req = handshake_req.clone();
        let peer = *peer;
        let net = net.clone();
        async move {
            let timer = Instant::now();
            let result = net
                .bincode_post::<HandshakeRequest, HandshakeResponse>(
                    format!("http://{}/bincode/peers", peer),
                    handshake_req,
                    Limit::default().size(5 * KB).time(SECOND),
                )
                .await;
            result.map(|r| (r, timer.elapsed()))
        }
    })
    .await;

    {
        let mut ctx = context.write().await;
        for (p, resp) in peer_responses {
            if let Ok((resp, ping_time)) = resp {
                if p == resp.peer.address {
                    ctx.peer_manager.add_node(resp.peer, ping_time);
                } else {
                    // ?!
                }
            }
        }
    }
    Ok(())
}
