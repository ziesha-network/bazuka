use tokio::try_join;

use crate::cli::{get_conf, get_wallet_collection, get_wallet_path};
use crate::client::{BazukaClient, NodeError};
use crate::core::{Amount, Money, TokenId};
use crate::crypto::ed25519::PublicKey;

pub async fn delegate(memo: Option<String>, amount: Amount, to: PublicKey, fee: Amount) -> () {
    let wallet = get_wallet_collection();
    let wallet_path = get_wallet_path();
    let conf = get_conf();
    let (conf, mut wallet) = conf.zip(wallet).expect("Bazuka is not initialized!");
    let tx_builder = wallet.user_builder(0);
    let (req_loop, client) =
        BazukaClient::connect(tx_builder.get_priv_key(), conf.random_node(), conf.network);
    try_join!(
        async move {
            let curr_nonce = client
                .get_account(tx_builder.get_address())
                .await?
                .account
                .nonce;

            let new_nonce = wallet.user(0).new_r_nonce().unwrap_or(curr_nonce + 1);
            let tx = tx_builder.delegate(
                memo.unwrap_or_default(),
                to,
                amount,
                false,
                Money {
                    amount: fee,
                    token_id: TokenId::Ziesha,
                },
                new_nonce,
            );
            wallet.user(0).add_rsend(tx.clone());
            wallet.save(wallet_path).unwrap();
            println!("{:#?}", client.transact(tx).await?);
            Ok::<(), NodeError>(())
        },
        req_loop
    )
    .unwrap();
}
