use super::*;

pub async fn send_mining_puzzle<B: Blockchain>(
    context: &Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    let mut ctx = context.write().await;
    let net = ctx.outgoing.clone();

    if let Some(w) = ctx.wallet.clone() {
        let (blk, puzzle) = ctx.get_puzzle(w)?;
        if let Some(m) = &mut ctx.miner {
            if m.block_puzzle.is_none() {
                if let Some(webhook) = m.webhook.clone() {
                    net.json_post::<Puzzle, String>(
                        webhook.to_string(),
                        puzzle.clone(),
                        Limit::new().size(1024 * 1024).time(1000),
                    )
                    .await?;
                    m.block_puzzle = Some((blk, puzzle));
                }
            }
        }
    }
    Ok(())
}
