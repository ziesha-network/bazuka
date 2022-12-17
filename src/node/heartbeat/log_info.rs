use super::*;

use colored::Colorize;

pub async fn log_info<B: Blockchain>(
    context: &Arc<RwLock<NodeContext<B>>>,
) -> Result<(), NodeError> {
    let ctx = context.read().await;
    let mut inf = Vec::new();
    inf.extend([
        ("Height", ctx.blockchain.get_height()?.to_string()),
        (
            "Outdated states",
            ctx.blockchain.get_outdated_contracts()?.len().to_string(),
        ),
        ("Timestamp", ctx.network_timestamp().to_string()),
        (
            "Active nodes",
            ctx.peer_manager.get_peers().len().to_string(),
        ),
    ]);

    inf.push(("Chain Pool", ctx.mempool.chain_sourced.len().to_string()));
    inf.push(("MPN Pool", ctx.mempool.mpn_sourced.len().to_string()));

    let wallet_addr = ctx.wallet.get_address();
    let acc = ctx.blockchain.get_account(wallet_addr)?;
    inf.push(("Balance", acc.balance().to_string()));

    println!(
        "{}",
        inf.into_iter()
            .map(|(k, v)| format!("{}: {}", k.bright_blue(), v))
            .collect::<Vec<String>>()
            .join(" ")
    );

    Ok(())
}
