use futures::try_join;

use crate::cli::BazukaConfig;

use bazuka::{
    client::{BazukaClient, NodeError},
    wallet::WalletCollection,
};

pub async fn status(conf: Option<BazukaConfig>, wallet: Option<WalletCollection>) {
    let (conf, mut wallet) = conf.zip(wallet).expect("Bazuka is not initialized!");
    let wallet = wallet.user(0).tx_builder();
    let (req_loop, client) =
        BazukaClient::connect(wallet.get_priv_key(), conf.random_node(), conf.network);
    try_join!(
        async move {
            println!("{:#?}", client.stats().await?);
            Ok::<(), NodeError>(())
        },
        req_loop
    )
    .unwrap();
}
