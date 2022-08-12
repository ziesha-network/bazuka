use super::*;
use crate::common::*;

pub async fn sync_blocks<B: Blockchain>(
    context: &Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    let ctx = context.read().await;
    let net = ctx.outgoing.clone();
    let opts = ctx.opts.clone();
    let max_block_size = ctx.blockchain.config().max_block_size;
    let mut sorted_peers = ctx.active_peers();
    drop(ctx);

    sorted_peers.sort_by_key(|p| p.info.as_ref().map(|i| i.power).unwrap_or(0));

    for peer in sorted_peers.iter().rev() {
        if let Some(inf) = peer.info.as_ref() {
            let mut success = true;
            while success {
                let ctx = context.read().await;
                let min_target = ctx.blockchain.config().minimum_pow_difficulty;
                if inf.power <= ctx.blockchain.get_power()? {
                    return Ok(());
                }

                log::info!(
                    "Syncing blocks with: {} (Peer power: {})",
                    peer.address,
                    inf.power
                );

                let local_height = ctx.blockchain.get_height()?;
                let start_height = std::cmp::min(local_height, inf.height);
                drop(ctx);

                // WARN: Chain might change when getting responses from users, maybe get all data needed before dropping ctx

                // Get all headers starting from the indices that we don't have.
                let resp = net
                    .bincode_get::<GetHeadersRequest, GetHeadersResponse>(
                        format!("{}/bincode/headers", peer.address),
                        GetHeadersRequest {
                            since: start_height,
                            count: opts.max_blocks_fetch,
                        },
                        Limit::default()
                            .size(opts.max_blocks_fetch * KB)
                            .time(3 * SECOND),
                    )
                    .await?;

                let (mut headers, pow_keys) = (resp.headers, resp.pow_keys);

                // TODO: Check parent hashes
                let ctx = context.read().await;
                for (i, (head, pow_key)) in headers.iter().zip(pow_keys.into_iter()).enumerate() {
                    if head.proof_of_work.target < min_target || !head.meets_target(&pow_key) {
                        panic!("Header doesn't meet min target!");
                    }
                    if head.number != start_height + i as u64 {
                        panic!("Bad header number returned!");
                    }
                    if head.number < local_height
                        && head == &ctx.blockchain.get_header(head.number)?
                    {
                        panic!("Duplicate header given!");
                    }
                }
                drop(ctx);

                log::info!(
                    "Got headers {}-{}...",
                    start_height,
                    start_height + headers.len() as u64
                );

                // The local blockchain and the peer blockchain both have all blocks
                // from 0 to height-1, though, the blocks might not be equal. Find
                // the header from which the fork has happened.
                for index in (0..start_height).rev() {
                    let peer_resp = net
                        .bincode_get::<GetHeadersRequest, GetHeadersResponse>(
                            format!("{}/bincode/headers", peer.address),
                            GetHeadersRequest {
                                since: index,
                                count: 1,
                            },
                            Limit::default().size(1 * KB).time(1 * SECOND),
                        )
                        .await?;
                    let (peer_header, peer_pow_key) =
                        (peer_resp.headers[0].clone(), peer_resp.pow_keys[0].clone());

                    if peer_header.proof_of_work.target < min_target
                        || !peer_header.meets_target(&peer_pow_key)
                    {
                        panic!("Header doesn't meet min target!");
                    }
                    if peer_header.number != index as u64 {
                        panic!("Bad header number!");
                    }
                    if peer_header.hash() != headers[0].parent_hash {
                        panic!("Bad header hash!");
                    }

                    log::info!("Got header {}...", index);

                    let ctx = context.read().await;
                    let local_header = ctx.blockchain.get_headers(index, 1)?[0].clone();
                    drop(ctx);

                    if local_header.hash() != peer_header.hash() {
                        headers.insert(0, peer_header);
                    } else {
                        break;
                    }
                }

                let will_extend = {
                    let ctx = context.write().await;
                    let banned = headers.iter().any(|h| ctx.banned_headers.contains_key(h));

                    if banned {
                        log::warn!("Chain has banned headers!");
                    }

                    let will_extend =
                        match ctx
                            .blockchain
                            .will_extend(headers[0].number, &headers, true)
                        {
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

                    !banned && will_extend
                };

                if will_extend {
                    let resp = net
                        .bincode_get::<GetBlocksRequest, GetBlocksResponse>(
                            format!("{}/bincode/blocks", peer.address).to_string(),
                            GetBlocksRequest {
                                since: headers[0].number,
                                count: opts.max_blocks_fetch,
                            },
                            Limit::default()
                                .size(opts.max_blocks_fetch as u64 * max_block_size as u64 * 2)
                                .time(opts.max_blocks_fetch as u32 * 30 * SECOND),
                        )
                        .await?;
                    let mut ctx = context.write().await;

                    ctx.blockchain.extend(headers[0].number, &resp.blocks)?;
                    ctx.outdated_since = None;
                } else {
                    let mut ctx = context.write().await;
                    ctx.punish(peer.address, opts.incorrect_power_punish);
                    success = false;
                }
            }
        }
    }

    Ok(())
}
