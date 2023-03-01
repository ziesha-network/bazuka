use super::*;
use crate::common::*;
use std::time::{Duration, Instant};

pub async fn sync_clock<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    let ctx = context.read().await;

    let handshake_req = match ctx.get_info()? {
        Some(peer) => HandshakeRequest::Node(peer.address),
        None => HandshakeRequest::Client,
    };

    let net = ctx.outgoing.clone();

    let peer_addresses = ctx.peer_manager.get_peers();
    drop(ctx);

    log::info!("Syncing clocks...");
    let peer_responses: Vec<(Peer, Result<(HandshakeResponse, Duration), NodeError>)> =
        http::group_request(&peer_addresses, move |peer| {
            let handshake_req = handshake_req.clone();
            let peer = peer.clone();
            let net = net.clone();
            async move {
                let timer = Instant::now();
                let result = net
                    .bincode_post::<HandshakeRequest, HandshakeResponse>(
                        format!("http://{}/bincode/peers", peer.address),
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
        let resps = punish_non_responding(&mut ctx, &peer_responses)
            .into_iter()
            .collect::<Vec<_>>();
        for (p, (resp, ping_time)) in resps.iter() {
            if *p == resp.peer.address {
                ctx.peer_manager.add_node(resp.peer.clone(), *ping_time);
            }
        }
        let timestamps = resps
            .iter()
            .map(|(_, (resp, _))| resp.timestamp)
            .collect::<Vec<_>>();
        if !timestamps.is_empty() {
            // Set timestamp_offset according to median timestamp of the network
            let median_timestamp = utils::median(&timestamps);
            ctx.timestamp_offset = median_timestamp as i32 - utils::local_timestamp() as i32;
        }

        let mut accepted_claim = None;
        for (_, (resp, _)) in resps.iter() {
            if let Some(claim) = resp.validator_claim.clone() {
                if ctx.update_validator_claim(claim.clone())? {
                    accepted_claim = Some(claim);
                    break;
                }
            }
        }
        if let Some(claim) = accepted_claim {
            drop(ctx);
            promote_validator_claim(context, claim).await;
        }
    }
    Ok(())
}
