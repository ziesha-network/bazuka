use crate::cli::{get_conf, get_wallet_collection, get_wallet_path, BazukaConfig};
use bazuka::client::{BazukaClient, NodeError};


use bazuka::wallet::WalletCollection;
use tokio::try_join;

#[cfg(feature = "client")]
#[allow(dead_code)]
async fn resend_all_wallet_txs(
    conf: BazukaConfig,
    wallet: &mut WalletCollection,
) -> Result<(), NodeError> {
    let tx_builder = wallet.user(0).tx_builder();
    let (req_loop, client) =
        BazukaClient::connect(tx_builder.get_priv_key(), conf.random_node(), conf.network);
    try_join!(
        async move {
            for (_, txs) in wallet.user(0).txs.iter() {
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

pub async fn resend_pending() -> () {
    let wallet = get_wallet_collection();
    let wallet_path = get_wallet_path();
    let conf = get_conf();
    let (conf, mut wallet) = conf.zip(wallet).expect("Bazuka is not initialized!");
    resend_all_wallet_txs(conf, &mut wallet).await.unwrap();
    wallet.save(wallet_path).unwrap();
}
