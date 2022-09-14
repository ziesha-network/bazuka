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
            "Active peers",
            ctx.peer_manager.get_peers().len().to_string(),
        ),
    ]);

    inf.push(("Power", ctx.blockchain.get_power()?.to_string()));

    inf.push(("Tx Pool", ctx.mempool.len().to_string()));
    inf.push(("Tx/Zk Pool", ctx.contract_payment_mempool.len().to_string()));
    inf.push(("Zk Pool", ctx.zero_mempool.len().to_string()));

    if let Some(wallet) = ctx.wallet.clone() {
        let acc = ctx.blockchain.get_account(wallet.get_address())?;
        inf.push(("Balance", acc.balance.to_string()));
    }

    // TODO: Embed MPN in test environment
    #[cfg(not(test))]
    {
        let mpn_contract_id = ctx.blockchain.config().mpn_contract_id;
        let mpn_account = ctx.blockchain.get_contract_account(mpn_contract_id)?;
        inf.push(("MPN Height", mpn_account.height.to_string()));
        inf.push(("MPN Balance", mpn_account.balance.to_string()));
        inf.push((
            "MPN Size",
            mpn_account.compressed_state.state_size.to_string(),
        ));
    }

    println!(
        "{}",
        inf.into_iter()
            .map(|(k, v)| format!("{}: {}", k.bright_blue(), v))
            .collect::<Vec<String>>()
            .join(" ")
    );

    Ok(())
}
