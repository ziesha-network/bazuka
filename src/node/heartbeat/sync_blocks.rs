use super::*;

pub async fn sync_blocks<B: Blockchain>(
    context: &Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    let ctx = context.read().await;
    let net = ctx.outgoing.clone();
    let opts = ctx.opts.clone();
    let mut sorted_peers = ctx.active_peers();
    drop(ctx);

    sorted_peers.sort_by_key(|p| p.info.as_ref().map(|i| i.power).unwrap_or(0));

    for peer in sorted_peers.iter().rev() {
        if let Some(inf) = peer.info.as_ref() {
            let ctx = context.read().await;
            if inf.power <= ctx.blockchain.get_power()? {
                return Ok(());
            }
            let start_height = std::cmp::min(ctx.blockchain.get_height()?, inf.height);
            drop(ctx);

            // Get all headers starting from the indices that we don't have.
            let mut headers = net
                .bincode_get::<GetHeadersRequest, GetHeadersResponse>(
                    format!("{}/bincode/headers", peer.address),
                    GetHeadersRequest {
                        since: start_height,
                        until: None,
                    },
                    Limit::default().size(1024 * 1024).time(1000),
                )
                .await?
                .headers;

            // The local blockchain and the peer blockchain both have all blocks
            // from 0 to height-1, though, the blocks might not be equal. Find
            // the header from which the fork has happened.
            for index in (0..start_height).rev() {
                let peer_header = net
                    .bincode_get::<GetHeadersRequest, GetHeadersResponse>(
                        format!("{}/bincode/headers", peer.address),
                        GetHeadersRequest {
                            since: index,
                            until: Some(index + 1),
                        },
                        Limit::default().size(1024 * 1024).time(1000),
                    )
                    .await?
                    .headers[0]
                    .clone();

                let ctx = context.read().await;
                let local_header = ctx.blockchain.get_headers(index, Some(index + 1))?[0].clone();
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

                !banned
                    && ctx
                        .blockchain
                        .will_extend(headers[0].number, &headers, true)
                        .unwrap_or(false)
            };

            if will_extend {
                let resp = net
                    .bincode_get::<GetBlocksRequest, GetBlocksResponse>(
                        format!("{}/bincode/blocks", peer.address).to_string(),
                        GetBlocksRequest {
                            since: headers[0].number,
                            until: None,
                        },
                        Limit::default().size(1024 * 1024).time(1000),
                    )
                    .await?;
                let mut ctx = context.write().await;

                ctx.blockchain.extend(headers[0].number, &resp.blocks)?;
                ctx.outdated_since = None;
            } else {
                let mut ctx = context.write().await;
                ctx.punish(peer.address, opts.incorrect_power_punish);
            }
        }
    }

    Ok(())
}
