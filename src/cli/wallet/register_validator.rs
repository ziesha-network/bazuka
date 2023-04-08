use std::path::PathBuf;

use tokio::try_join;

use crate::cli::BazukaConfig;
use bazuka::client::{BazukaClient, NodeError};
use bazuka::core::{Decimal, Money, NonceGroup, TokenId};
use bazuka::wallet::WalletCollection;

pub async fn register_validator(
    memo: Option<String>,
    commision: f32,
    fee: Decimal,
    conf: BazukaConfig,
    mut wallet: WalletCollection,
    wallet_path: &PathBuf,
) -> () {
    // TODO: Dirty code!
    if !(0.0..0.1).contains(&commision) {
        panic!("Commision out of range! Commision should be a float number between 0.0 to 0.1!");
    }

    let commision_u8 = (commision * (u8::MAX as f32)) as u8;
    let tx_builder = wallet.validator().tx_builder();
    let (req_loop, client) =
        BazukaClient::connect(tx_builder.get_priv_key(), conf.random_node(), conf.network);
    try_join!(
        async move {
            let curr_nonce = client.get_account(tx_builder.get_address()).await?.nonce;

            let new_nonce = wallet
                .validator()
                .new_nonce(NonceGroup::TransactionAndDelta(tx_builder.get_address()))
                .unwrap_or(curr_nonce + 1);
            let tx = tx_builder.register_validator(
                memo.unwrap_or_default(),
                commision_u8,
                Money {
                    amount: fee.to_amount(bazuka::config::UNIT_ZEROS),
                    token_id: TokenId::Ziesha,
                },
                new_nonce,
            );
            if let Some(err) = client.transact(tx.clone().into()).await?.error {
                println!("Error: {}", err);
            } else {
                wallet.validator().add_tx(tx.clone().into());
                wallet.save(wallet_path).unwrap();
                println!("Sent");
            }
            Ok::<(), NodeError>(())
        },
        req_loop
    )
    .unwrap();
}
