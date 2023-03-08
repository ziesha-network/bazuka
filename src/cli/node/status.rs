use futures::try_join;

use crate::{
    cli::{get_conf, get_wallet},
    client::{BazukaClient, NodeError},
    wallet::TxBuilder,
};

pub async fn status() {
    let conf = get_conf();
    let wallet = get_wallet();
    let (conf, wallet) = conf.zip(wallet).expect("Bazuka is not initialized!");
    let wallet = TxBuilder::new(&wallet.seed());
    let (req_loop, client) = BazukaClient::connect(
        wallet.get_priv_key(),
        conf.random_node(),
        conf.network,
        None,
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
