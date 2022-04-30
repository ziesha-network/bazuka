use super::*;

pub async fn sync_blocks<B: Blockchain>(
    context: &Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    let ctx = context.read().await;
    let height = ctx.blockchain.get_height()?;
    let peer_addresses = ctx
        .random_peers(&mut rand::thread_rng(), NUM_PEERS)
        .keys()
        .cloned()
        .collect::<Vec<PeerAddress>>();
    drop(ctx);

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
        let resps = punish_non_responding(&mut ctx, &header_responses).await;
        for (peer, resp) in resps.iter() {
            if !resp.headers.is_empty() {
                if ctx
                    .blockchain
                    .will_extend(height, &resp.headers)
                    .unwrap_or(false)
                {
                    println!("{} has a longer chain!", peer);
                    let resp = http::bincode_get::<GetBlocksRequest, GetBlocksResponse>(
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
