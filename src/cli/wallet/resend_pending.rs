use crate::cli::{BazukaConfig, CURRENT_NETWORK};
use bazuka::client::{BazukaClient, Limit, NodeError};
use bazuka::common::*;

use bazuka::wallet::WalletCollection;
use std::path::PathBuf;
use tokio::try_join;

#[cfg(feature = "client")]
#[allow(dead_code)]
async fn resend_all_wallet_txs(
    conf: BazukaConfig,
    wallet: &mut WalletCollection,
) -> Result<(), NodeError> {
    let tx_builder = wallet.user(0).tx_builder();
    let (req_loop, client) = BazukaClient::connect(
        tx_builder.get_priv_key(),
        conf.random_node(),
        CURRENT_NETWORK.into(),
        Some(Limit::default().time(2 * SECOND)),
    );
    try_join!(
        async move {
            for (_, txs) in wallet.user(0).txs.iter() {
                for tx in txs {
                    client.transact(tx.clone()).await?;
                }
            }
            for (_, txs) in wallet.validator().txs.iter() {
                for tx in txs {
                    client.transact(tx.clone()).await?;
                }
            }
            Ok::<(), NodeError>(())
        },
        req_loop
    )
    .unwrap();

    Ok(())
}

pub async fn resend_pending(
    conf: BazukaConfig,
    mut wallet: WalletCollection,
    wallet_path: &PathBuf,
) -> () {
    resend_all_wallet_txs(conf, &mut wallet).await.unwrap();
    wallet.save(wallet_path).unwrap();
}
