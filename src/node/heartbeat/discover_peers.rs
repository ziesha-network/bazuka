use super::*;

pub async fn discover_peers<B: Blockchain>(
    context: &Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    let _ctx = context.read().await;
    Ok(())
}
