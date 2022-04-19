use super::api::messages::*;
use super::{http, NodeContext, NodeError, PeerAddress};
use crate::blockchain::Blockchain;
use crate::config::punish;
use crate::utils;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

const NUM_PEERS: usize = 8;

pub async fn heartbeat<B: Blockchain>(
    address: PeerAddress,
    context: Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    loop {
        let mut ctx = context.write().await;
        let timestamp = ctx.network_timestamp();
        let info = ctx.get_info()?;
        let height = ctx.blockchain.get_height()?;
        let peer_addresses = ctx
            .random_peers(&mut rand::thread_rng(), NUM_PEERS)
            .keys()
            .cloned()
            .collect::<Vec<PeerAddress>>();
        drop(ctx);

        let peer_responses: Vec<(PeerAddress, Result<PostPeerResponse, NodeError>)> =
            http::group_request(&peer_addresses, |peer| {
                http::json_post::<PostPeerRequest, PostPeerResponse>(
                    format!("{}/peers", peer).to_string(),
                    PostPeerRequest {
                        address: address.clone(),
                        timestamp,
                        info: info.clone(),
                    },
                )
            })
            .await;

        {
            let mut ctx = context.write().await;
            for bad_peer in peer_responses
                .iter()
                .filter(|(_, resp)| resp.is_err())
                .map(|(p, _)| p)
            {
                ctx.peers
                    .entry(bad_peer.clone())
                    .and_modify(|stats| stats.punish(punish::NO_RESPONSE_PUNISH));
            }
            let timestamps = peer_responses
                .iter()
                .filter_map(|r| r.1.as_ref().ok())
                .map(|r| r.timestamp)
                .collect::<Vec<_>>();
            if timestamps.len() > 0 {
                // Set timestamp_offset according to median timestamp of the network
                let median_timestamp = utils::median(&timestamps);
                ctx.timestamp_offset = median_timestamp as i64 - utils::local_timestamp() as i64;
            }
            let active_peers = ctx.active_peers().len();
            println!(
                "Lub dub! (Height: {}, Network Timestamp: {}, Peers: {})",
                height,
                ctx.network_timestamp(),
                active_peers
            );
        }

        let header_responses: Vec<(PeerAddress, Result<GetHeadersResponse, NodeError>)> =
            http::group_request(&peer_addresses, |peer| {
                http::bincode_get::<GetHeadersRequest, GetHeadersResponse>(
                    format!("{}/bincode/headers", peer).to_string(),
                    GetHeadersRequest {
                        since: height,
                        until: None,
                    },
                )
            })
            .await;

        {
            let mut ctx = context.write().await;
            for bad_peer in header_responses
                .iter()
                .filter(|(_, resp)| resp.is_err())
                .map(|(p, _)| p)
            {
                ctx.peers
                    .entry(bad_peer.clone())
                    .and_modify(|stats| stats.punish(punish::NO_RESPONSE_PUNISH));
            }
            let resps = header_responses
                .into_iter()
                .filter_map(|r| {
                    if r.1.as_ref().is_ok() {
                        Some((r.0.clone(), r.1.unwrap()))
                    } else {
                        None
                    }
                })
                .collect::<Vec<(PeerAddress, GetHeadersResponse)>>();
            for (peer, resp) in resps.iter() {
                if !resp.headers.is_empty() {
                    if ctx.blockchain.will_extend(&resp.headers)? {
                        println!("{} has a longer chain!", peer);
                        let resp = http::bincode_get::<GetBlocksRequest, GetBlocksResponse>(
                            format!("{}/bincode/blocks", peer).to_string(),
                            GetBlocksRequest {
                                since: height,
                                until: None,
                            },
                        )
                        .await?;
                        ctx.blockchain.extend(height, &resp.blocks)?;
                    }
                }
            }
        }

        sleep(Duration::from_millis(1000)).await;
    }
}
