use super::*;

pub async fn sync_state<B: Blockchain>(
    context: &Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    let ctx = context.read().await;

    let net = ctx.outgoing.clone();

    let height = ctx.blockchain.get_height()?;
    let last_header_hash = ctx.blockchain.get_tip()?.hash();
    let state_height = ctx.blockchain.get_state_height()?;

    if state_height != height {
        // Find clients which their state_height is equal with our height
        let same_height_peers = ctx.active_peers().into_iter().filter(|p| {
            p.info
                .as_ref()
                .map(|i| i.state_height == height)
                .unwrap_or(false)
        });
        drop(ctx);

        for peer in same_height_peers {
            let patch = net
                .bincode_get::<GetStatesRequest, GetStatesResponse>(
                    format!("{}/bincode/states", peer.address),
                    GetStatesRequest {
                        from: state_height,
                        to: hex::encode(last_header_hash),
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
