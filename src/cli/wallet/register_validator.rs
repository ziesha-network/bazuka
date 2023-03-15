use std::path::PathBuf;

use tokio::try_join;

use crate::cli::BazukaConfig;
use crate::client::{BazukaClient, NodeError};
use crate::core::{Amount, Money, TokenId};
use crate::wallet::{TxBuilder, Wallet};

pub async fn register_validator(
    memo: Option<String>,
    commision: f32,
    fee: Amount,
    conf: Option<BazukaConfig>,
    wallet: Option<Wallet>,
    wallet_path: &PathBuf,
) -> () {
    // TODO: Dirty code!
    if (0.0..0.1).contains(&commision) {
        panic!("Commision out of range!");
    }

    let commision_u8 = (commision * (u8::MAX as f32)) as u8;
    let (conf, mut wallet) = conf.zip(wallet).expect("Bazuka is not initialized!");
    let tx_builder = TxBuilder::new(&wallet.seed());
    let (req_loop, client) = BazukaClient::connect(
        tx_builder.get_priv_key(),
        conf.random_node(),
        conf.network,
        None,
    );
    try_join!(
        async move {
            let curr_nonce = client
                .get_account(tx_builder.get_address())
                .await?
                .account
                .nonce;

            let new_nonce = wallet.new_r_nonce().unwrap_or(curr_nonce + 1);
            let tx = tx_builder.register_validator(
                memo.unwrap_or_default(),
                commision_u8,
                Money {
                    amount: fee,
                    token_id: TokenId::Ziesha,
                },
                new_nonce,
            );
            wallet.add_rsend(tx.clone());
            wallet.save(wallet_path).unwrap();
            println!("{:#?}", client.transact(tx).await?);
            Ok::<(), NodeError>(())
        },
        req_loop
    )
    .unwrap();
}
