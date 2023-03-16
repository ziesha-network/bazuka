use std::path::PathBuf;

use tokio::try_join;

use crate::cli::BazukaConfig;
use crate::client::{BazukaClient, NodeError};
use crate::core::{Amount, Money, TokenId};
use crate::wallet::WalletCollection;

pub async fn register_validator(
    memo: Option<String>,
    commision: f32,
    fee: Amount,
    conf: Option<BazukaConfig>,
    wallet: Option<WalletCollection>,
    wallet_path: &PathBuf,
) -> () {
    // TODO: Dirty code!
    if !(0.0..0.1).contains(&commision) {
        panic!("Commision out of range! Commision should be a float number between 0.0 to 0.1!");
    }

    let commision_u8 = (commision * (u8::MAX as f32)) as u8;
    let (conf, mut wallet) = conf.zip(wallet).expect("Bazuka is not initialized!");
    let tx_builder = wallet.validator_builder();
    let (req_loop, client) =
        BazukaClient::connect(tx_builder.get_priv_key(), conf.random_node(), conf.network);
    try_join!(
        async move {
            let curr_nonce = client
                .get_account(tx_builder.get_address())
                .await?
                .account
                .nonce;

            let new_nonce = wallet.validator().new_r_nonce().unwrap_or(curr_nonce + 1);
            let tx = tx_builder.register_validator(
                memo.unwrap_or_default(),
                commision_u8,
                Money {
                    amount: fee,
                    token_id: TokenId::Ziesha,
                },
                new_nonce,
            );
            wallet.validator().add_rsend(tx.clone());
            wallet.save(wallet_path).unwrap();
            println!("{:#?}", client.transact(tx).await?);
            Ok::<(), NodeError>(())
        },
        req_loop
    )
    .unwrap();
}
