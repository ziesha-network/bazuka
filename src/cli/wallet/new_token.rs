use std::path::PathBuf;

use crate::cli::BazukaConfig;
use bazuka::client::{BazukaClient, NodeError};
use bazuka::core::{Amount, Money, NonceGroup, TokenId};
use bazuka::wallet::WalletCollection;
use tokio::try_join;

pub async fn new_token(
    memo: Option<String>,
    name: String,
    symbol: String,
    supply: Amount,
    decimals: u8,
    mintable: bool,
    fee: Amount,
    conf: Option<BazukaConfig>,
    wallet: Option<WalletCollection>,
    wallet_path: &PathBuf,
) -> () {
    let (conf, mut wallet) = conf.zip(wallet).expect("Bazuka is not initialized!");
    let tx_builder = wallet.user(0).tx_builder();
    let (req_loop, client) =
        BazukaClient::connect(tx_builder.get_priv_key(), conf.random_node(), conf.network);
    try_join!(
        async move {
            let curr_nonce = client
                .get_account(tx_builder.get_address())
                .await?
                .account
                .nonce;

            let new_nonce = wallet
                .user(0)
                .new_nonce(NonceGroup::TransactionAndDelta(tx_builder.get_address()))
                .unwrap_or(curr_nonce + 1);
            let (pay, token_id) = tx_builder.create_token(
                memo.unwrap_or_default(),
                name,
                symbol,
                supply,
                decimals,
                mintable.then(|| tx_builder.get_address()),
                Money {
                    amount: fee,
                    token_id: TokenId::Ziesha,
                },
                new_nonce,
            );
            if let Some(err) = client.transact(pay.clone().into()).await?.error {
                println!("Error: {}", err);
            } else {
                wallet.user(0).add_token(token_id);
                wallet.user(0).add_tx(pay.clone().into());
                wallet.save(wallet_path).unwrap();
                println!("Sent");
                println!("Token-Id: {}", token_id);
            }
            Ok::<(), NodeError>(())
        },
        req_loop
    )
    .unwrap();
}
