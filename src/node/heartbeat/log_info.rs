use super::*;

pub async fn log_info<B: Blockchain, N: Network>(
    context: &Arc<RwLock<NodeContext<N, B>>>,
) -> Result<(), NodeError> {
    let ctx = context.read().await;
    let mut inf = Vec::new();
    inf.extend([
        (
            "Height".to_string(),
            ctx.blockchain.get_height()?.to_string(),
        ),
        ("Timestamp".to_string(), ctx.network_timestamp().to_string()),
        (
            "Active peers".to_string(),
            ctx.active_peers().len().to_string(),
        ),
    ]);
    #[cfg(feature = "pow")]
    inf.push(("Power".to_string(), ctx.blockchain.get_power()?.to_string()));
    println!("Lub dub! {:?}", inf);

    Ok(())
}
