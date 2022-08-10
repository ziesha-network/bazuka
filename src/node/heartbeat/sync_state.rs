use super::*;
use crate::common::*;

pub async fn sync_state<B: Blockchain>(
    context: &Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    let mut ctx = context.write().await;

    let net = ctx.outgoing.clone();

    let ts = ctx.network_timestamp();

    let outdated_heights = ctx.blockchain.get_outdated_heights()?;
    if !outdated_heights.is_empty() && ctx.outdated_since.is_none() {
        ctx.outdated_since = Some(ts);
    } else if outdated_heights.is_empty() && ctx.outdated_since.is_some() {
        ctx.outdated_since = None;
    }

    if !outdated_heights.is_empty() {
        if let Some(outdated_since) = ctx.outdated_since {
            if (ts as i64 - outdated_since as i64) > ctx.opts.outdated_heights_threshold as i64 {
                let tip = ctx.blockchain.get_tip()?;
                ctx.banned_headers.insert(tip, ts);
                ctx.blockchain.rollback()?;
                ctx.outdated_since = None;
                return Ok(());
            }
        }
        let height = ctx.blockchain.get_height()?;
        let last_header = ctx.blockchain.get_tip()?;

        // Find clients which their height is equal with our height
        let same_height_peers = ctx
            .active_peers()
            .into_iter()
            .filter(|p| p.info.as_ref().map(|i| i.height == height).unwrap_or(false));
        drop(ctx);

        for peer in same_height_peers {
            let patch = net
                .bincode_get::<GetStatesRequest, GetStatesResponse>(
                    format!("{}/bincode/states", peer.address),
                    GetStatesRequest {
                        outdated_heights: outdated_heights.clone(),
                        to: hex::encode(last_header.hash()),
                    },
                    Limit::default().time(30 * MINUTE),
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
