use super::*;

pub async fn cleanup_mempool<B: Blockchain>(
    context: &Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    let mut ctx = context.write().await;
    ctx.cleanup_mempool()?;
    Ok(())
}
