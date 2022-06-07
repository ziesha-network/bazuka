use super::*;

pub async fn sync_state<B: Blockchain>(
    context: &Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    let mut ctx = context.write().await;

    let net = ctx.outgoing.clone();

    let ts = ctx.network_timestamp();
    let height = ctx.blockchain.get_height()?;
    let last_header = ctx.blockchain.get_tip()?;
    let outdated_states = ctx.blockchain.get_outdated_states_request()?;
    if !outdated_states.is_empty() && ctx.outdated_since.is_none() {
        ctx.outdated_since = Some(ts);
    } else if outdated_states.is_empty() && ctx.outdated_since.is_some() {
        ctx.outdated_since = None;
    }
    // Find clients which their height is equal with our height
    let same_height_peers = ctx
        .active_peers()
        .into_iter()
        .filter(|p| p.info.as_ref().map(|i| i.height == height).unwrap_or(false));

    if !outdated_states.is_empty() {
        if let Some(outdated_since) = ctx.outdated_since {
            if (ts as i64 - outdated_since as i64) > ctx.opts.outdated_states_threshold as i64 {
                ctx.banned_headers.push(last_header);
                ctx.blockchain.rollback()?;
                ctx.outdated_since = None;
                return Ok(());
            }
        }

        drop(ctx);
        for peer in same_height_peers {
            let patch = net
                .bincode_get::<GetStatesRequest, GetStatesResponse>(
                    format!("{}/bincode/states", peer.address),
                    GetStatesRequest {
                        outdated_states: outdated_states.clone(),
                        to: hex::encode(last_header.hash()),
                    },
                    Limit::default().size(1024 * 1024).time(1000),
                )
                .await?
                .patch;
            let mut ctx = context.write().await;
            if ctx.blockchain.update_states(&patch).is_ok() {
                break;
            }
        }
    }
    Ok(())
}
