use super::*;

pub async fn refresh<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
) -> Result<(), NodeError> {
    let mut ctx = context.write().await;
    let opts = ctx.opts.clone();
    ctx.refresh()?;
    if ctx.peer_manager.get_peers().len() < opts.num_peers {
        ctx.peer_manager.select_peers(opts.num_peers);
    }
    Ok(())
}
