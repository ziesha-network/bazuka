use super::*;

pub async fn refresh<B: Blockchain>(context: Arc<RwLock<NodeContext<B>>>) -> Result<(), NodeError> {
    let mut ctx = context.write().await;
    ctx.refresh()?;
    Ok(())
}
