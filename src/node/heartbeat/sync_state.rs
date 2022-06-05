use super::*;

pub async fn sync_state<B: Blockchain>(
    context: &Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    let ctx = context.read().await;

    let net = ctx.outgoing.clone();

    let ts = ctx.network_timestamp();
    let height = ctx.blockchain.get_height()?;
    let last_header = ctx.blockchain.get_tip()?;
    let outdated_states = ctx.blockchain.get_outdated_states_request()?;
    // Find clients which their height is equal with our height
    let same_height_peers = ctx
        .active_peers()
        .into_iter()
        .filter(|p| p.info.as_ref().map(|i| i.height == height).unwrap_or(false));
    drop(ctx);

    // TODO: Put better rollback condition
    if (ts as i64 - last_header.proof_of_work.timestamp as i64) > 200 && !outdated_states.is_empty()
    {
        let mut ctx = context.write().await;
        ctx.banned_headers.push(last_header);
        ctx.blockchain.rollback()?;
    } else if !outdated_states.is_empty() {
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
