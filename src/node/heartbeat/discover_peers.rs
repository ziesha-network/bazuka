use super::*;
use crate::common::*;

pub async fn discover_peers<B: Blockchain>(
    context: &Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    let ctx = context.read().await;

    let handshake_req = match ctx.get_info()? {
        Some(peer) => HandshakeRequest::Node(peer),
        None => HandshakeRequest::Client,
    };

    let opts = ctx.opts.clone();

    let net = ctx.outgoing.clone();

    let peer_addresses = ctx.peer_manager.random_candidates(opts.num_peers);
    drop(ctx);

    let peer_responses: Vec<(PeerAddress, Result<HandshakeResponse, NodeError>)> =
        http::group_request(&peer_addresses, |peer| {
            net.json_post::<HandshakeRequest, HandshakeResponse>(
                format!("{}/peers", peer),
                handshake_req.clone(),
                Limit::default().size(1 * KB).time(1 * SECOND),
            )
        })
        .await;

    {
        let mut ctx = context.write().await;
        for (p, resp) in peer_responses {
            if let Ok(resp) = resp {
                if p == resp.peer.address {
                    ctx.peer_manager.add_peer(resp.peer);
                } else {
                    // ?!
                }
            }
        }
    }
    Ok(())
}
