use super::*;
use crate::common::*;

pub async fn sync_blocks<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
) -> Result<(), NodeError> {
    let ctx = context.read().await;
    let net = ctx.outgoing.clone();
    let opts = ctx.opts.clone();
    let max_block_size = ctx.blockchain.config().max_block_size;
    let mut sorted_peers = ctx.peer_manager.get_peers();
    drop(ctx);

    sorted_peers.retain(|p| !p.power.is_nan());
    sorted_peers.sort_by(|a, b| a.power.partial_cmp(&b.power).unwrap());

    for peer in sorted_peers.iter().rev() {
        let mut net_fail = false;
        let mut chain_fail = false;
        loop {
            let ctx = context.read().await;
            if peer.power <= ctx.blockchain.get_power()? {
                return Ok(());
            }

            println!(
                "Syncing blocks with: {} (Peer height: {}, power: {})",
                peer.address, peer.height, peer.power
            );

            let local_height = ctx.blockchain.get_height()?;
            let start_height = std::cmp::min(local_height, peer.height);
            drop(ctx);

            // WARN: Chain might change when getting responses from users, maybe get all data needed before dropping ctx

            // Get all headers starting from the indices that we don't have.
            let resp = if let Ok(resp) = net
                .bincode_get::<GetHeadersRequest, GetHeadersResponse>(
                    format!("http://{}/bincode/headers", peer.address),
                    GetHeadersRequest {
                        since: start_height,
                        count: opts.max_blocks_fetch,
                    },
                    Limit::default()
                        .size(opts.max_blocks_fetch * KB)
                        .time(5 * SECOND),
                )
                .await
            {
                resp
            } else {
                net_fail = true;
                break;
            };

            let mut headers = resp.headers;

            if headers.is_empty() {
                log::warn!("Peer returned no headers!");
                chain_fail = true;
                break;
            }
            // TODO: Check parent hashes
            let ctx = context.read().await;
            let net_ts = ctx.network_timestamp();
            let max_ts_diff = ctx.opts.max_block_time_difference;
            for (i, head) in headers.iter().enumerate() {
                if head.number != start_height + i as u64 {
                    log::warn!("Bad header number returned!");
                    chain_fail = true;
                    break;
                }
                if head.proof_of_stake.timestamp.saturating_sub(net_ts) > max_ts_diff {
                    log::warn!("Block timestamp is way ahead of future!");
                    chain_fail = true;
                    break;
                }
                if head.number < local_height && head == &ctx.blockchain.get_header(head.number)? {
                    log::warn!("Duplicate header given!");
                    chain_fail = true;
                    break;
                }
            }
            drop(ctx);

            if chain_fail || net_fail {
                break;
            }

            log::info!(
                "Got headers {}-{}...",
                start_height,
                start_height + headers.len() as u64
            );

            // The local blockchain and the peer blockchain both have all blocks
            // from 0 to height-1, though, the blocks might not be equal. Find
            // the header from which the fork has happened.
            for index in (0..start_height).rev() {
                let peer_resp = if let Ok(resp) = net
                    .bincode_get::<GetHeadersRequest, GetHeadersResponse>(
                        format!("http://{}/bincode/headers", peer.address),
                        GetHeadersRequest {
                            since: index,
                            count: 1,
                        },
                        Limit::default().size(KB).time(3 * SECOND),
                    )
                    .await
                {
                    resp
                } else {
                    net_fail = true;
                    break;
                };
                if peer_resp.headers.is_empty() {
                    log::warn!("Peer is not providing claimed headers!");
                    chain_fail = true;
                    break;
                }

                let peer_header = peer_resp.headers[0].clone();

                if peer_header.number != index as u64 {
                    log::warn!("Bad header number!");
                    chain_fail = true;
                    break;
                }
                if peer_header.proof_of_stake.timestamp.saturating_sub(net_ts) > max_ts_diff {
                    log::warn!("Block timestamp is way ahead of future!");
                    chain_fail = true;
                    break;
                }
                if peer_header.hash() != headers[0].parent_hash {
                    log::warn!("Bad header hash!");
                    chain_fail = true;
                    break;
                }

                log::info!("Got header {}...", index);

                let ctx = context.read().await;
                let local_header = ctx.blockchain.get_headers(index, 1)?.first().cloned();
                drop(ctx);

                if let Some(local_header) = local_header {
                    if local_header.hash() != peer_header.hash() {
                        headers.insert(0, peer_header);
                    } else {
                        break;
                    }
                } else {
                    chain_fail = true;
                    break;
                }
            }

            if chain_fail || net_fail {
                break;
            }

            let ctx = context.read().await;

            let will_extend = match ctx.blockchain.will_extend(headers[0].number, &headers) {
                Ok(result) => {
                    if !result {
                        log::warn!("Chain is not powerful enough!");
                    }
                    result
                }
                Err(e) => {
                    log::warn!("Chain is invalid! Error: {}", e);
                    false
                }
            };

            if !will_extend {
                chain_fail = true;
                break;
            }

            drop(ctx);

            if let Ok(resp) = net
                .bincode_get::<GetBlocksRequest, GetBlocksResponse>(
                    format!("http://{}/bincode/blocks", peer.address).to_string(),
                    GetBlocksRequest {
                        since: headers[0].number,
                        count: opts.max_blocks_fetch,
                    },
                    Limit::default()
                        .size(opts.max_blocks_fetch as u64 * max_block_size as u64 * 2)
                        .time(opts.max_blocks_fetch as u32 * 30 * SECOND),
                )
                .await
            {
                let mut ctx = context.write().await;

                match ctx.blockchain.extend(headers[0].number, &resp.blocks) {
                    Ok(_) => {
                        println!("Height advanced to {}!", ctx.blockchain.get_height()?);
                        ctx.on_update()?;
                    }
                    Err(e) => {
                        chain_fail = true;
                        log::warn!("Cannot extend the blockchain. Error: {}", e);
                        break;
                    }
                }
            } else {
                net_fail = true;
                log::warn!("Network error! Cannot fetch blocks...");
                break;
            }
        }
        if chain_fail {
            context.write().await.punish_bad_behavior(
                peer.address,
                opts.incorrect_chain_punish,
                "Cannot sync blocks!",
            );
        } else if net_fail {
            context.write().await.punish_unresponsive(peer.address);
        }
    }

    Ok(())
}
