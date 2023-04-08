use futures::try_join;

use crate::cli::{BazukaConfig, CURRENT_NETWORK};

use bazuka::{
    client::{BazukaClient, NodeError},
    wallet::WalletCollection,
};

pub async fn status(conf: BazukaConfig, mut wallet: WalletCollection) {
    let wallet = wallet.user(0).tx_builder();
    let (req_loop, client) = BazukaClient::connect(
        wallet.get_priv_key(),
        conf.random_node(),
        CURRENT_NETWORK.into(),
    );
    try_join!(
        async move {
            println!("{:#?}", client.stats().await?);
            Ok::<(), NodeError>(())
        },
        req_loop
    )
    .unwrap();
}
