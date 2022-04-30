use super::*;

pub async fn send_mining_puzzle<B: Blockchain>(
    context: &Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    let mut ctx = context.write().await;
    if let Some(w) = ctx.wallet.clone() {
        let (blk, puzzle) = ctx.get_puzzle(w)?;
        if let Some(m) = &mut ctx.miner {
            if m.block.is_none() {
                http::json_post::<Puzzle, String>(m.webhook.to_string(), puzzle).await?;
                m.block = Some(blk);
            }
        }
    }
    Ok(())
}
