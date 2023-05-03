use futures::try_join;

use crate::cli::{BazukaConfig, CURRENT_NETWORK};

use bazuka::{
    client::{BazukaClient, Limit, NodeError},
    common::*,
    wallet::WalletCollection,
};

pub async fn status(conf: BazukaConfig, mut wallet: WalletCollection) {
    let wallet = wallet.user(0).tx_builder();
    let (req_loop, client) = BazukaClient::connect(
        wallet.get_priv_key(),
        conf.random_node(),
        CURRENT_NETWORK.into(),
        Some(Limit::default().time(2 * SECOND)),
    );
    try_join!(
        async move {
            println!("{:#?}", client.stats().await.expect("Could not receive stats from the node. Make sure the node is started and running."));
            Ok::<(), NodeError>(())
        },
        req_loop
    )
    .unwrap();
}
