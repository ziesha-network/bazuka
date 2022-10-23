use super::*;
use crate::common::*;

pub async fn sync_state<B: Blockchain>(
    context: &Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    let mut ctx = context.write().await;

    let net = ctx.outgoing.clone();

    let ts = ctx.local_timestamp();

    let outdated_heights = ctx.blockchain.get_outdated_heights()?;
    if !outdated_heights.is_empty() && ctx.outdated_since.is_none() {
        ctx.outdated_since = Some(ts);
    } else if outdated_heights.is_empty() && ctx.outdated_since.is_some() {
        ctx.outdated_since = None;
    }

    log::info!("Syncing states...");
    if !outdated_heights.is_empty() {
        if let Some(outdated_since) = ctx.outdated_since {
            if (ts as i64 - outdated_since as i64) > ctx.opts.outdated_heights_threshold as i64 {
                let tip = ctx.blockchain.get_tip()?;
                ctx.banned_headers.insert(tip, ts);
                ctx.blockchain.rollback()?;
                ctx.on_update()?;
                return Ok(());
            }
        }
        let height = ctx.blockchain.get_height()?;
        let last_header = ctx.blockchain.get_tip()?;

        // Find clients which their height is equal with our height
        let same_height_peers = ctx
            .peer_manager
            .get_peers()
            .filter(|p| p.height == height)
            .cloned()
            .collect::<Vec<_>>();
        drop(ctx);

        for peer in same_height_peers {
            log::info!("Syncing state with {}...", peer.address);
            match net
                .bincode_get::<GetStatesRequest, GetStatesResponse>(
                    format!("{}/bincode/states", peer.address),
                    GetStatesRequest {
                        outdated_heights: outdated_heights.clone(),
                        to: hex::encode(last_header.hash()),
                    },
                    Limit::default().time(1 * MINUTE),
                )
                .await
            {
                Ok(resp) => match context.write().await.blockchain.update_states(&resp.patch) {
                    Ok(_) => {}
                    Err(e) => {
                        log::warn!("Wrong state-patch given! Error: {}", e);
                    }
                },
                Err(e) => {
                    log::warn!("Could not get state-patch! Error: {}", e);
                }
            }
        }
    }
    Ok(())
}
