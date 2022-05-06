use super::*;

pub async fn sync_blocks<N: Network, B: Blockchain>(
    context: &Arc<RwLock<NodeContext<N, B>>>,
) -> Result<(), NodeError> {
    let ctx = context.read().await;

    let net = Arc::clone(&ctx.network);

    let height = ctx.blockchain.get_height()?;
    let peer_addresses = ctx
        .random_peers(&mut rand::thread_rng(), NUM_PEERS)
        .into_iter()
        .map(|p| p.address)
        .collect::<Vec<PeerAddress>>();
    drop(ctx);

    let header_responses: Vec<(PeerAddress, Result<GetHeadersResponse, NodeError>)> =
        http::group_request(&peer_addresses, |peer| {
            net.bincode_get::<GetHeadersRequest, GetHeadersResponse>(
                format!("{}/bincode/headers", peer),
                GetHeadersRequest {
                    since: height,
                    until: None,
                },
            )
        })
        .await;

    {
        let mut ctx = context.write().await;
        let resps = punish_non_responding(&mut ctx, &header_responses);
        for (peer, resp) in resps.iter() {
            if !resp.headers.is_empty() {
                if ctx
                    .blockchain
                    .will_extend(height, &resp.headers)
                    .unwrap_or(false)
                {
                    println!("{} has a longer chain!", peer);
                    let resp = net
                        .bincode_get::<GetBlocksRequest, GetBlocksResponse>(
                            format!("{}/bincode/blocks", peer).to_string(),
                            GetBlocksRequest {
                                since: height,
                                until: None,
                            },
                        )
                        .await?;
                    if ctx.blockchain.extend(height, &resp.blocks).is_err() {
                        ctx.punish(*peer, punish::INVALID_DATA_PUNISH);
                    }
                } else {
                    ctx.punish(*peer, punish::INVALID_DATA_PUNISH);
                }
            }
        }
    }

    Ok(())
}
