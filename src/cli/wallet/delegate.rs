use tokio::try_join;

use crate::client::{BazukaClient, NodeError};
use crate::core::{Amount, TokenId, Money};
use crate::crypto::ed25519::PublicKey;
use crate::wallet::{TxBuilder};

use super::{get_wallet, get_wallet_path, get_conf};

pub async fn delegate(
    memo: Option<String>,
    amount: Amount,
    since: Option<u32>,
    count: u32,
    to: PublicKey,
    fee: Amount,
) -> () {
    let wallet = get_wallet();
    let wallet_path = get_wallet_path();
    let conf = get_conf();
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
            let epoch = client.stats().await?.epoch;
            let curr_nonce = client
                .get_account(tx_builder.get_address())
                .await?
                .account
                .nonce;

            let new_nonce = wallet.new_r_nonce().unwrap_or(curr_nonce + 1);
            let (delegate_id, tx) = tx_builder.delegate(
                memo.unwrap_or_default(),
                to,
                amount,
                since.unwrap_or(epoch + 1),
                count,
                Money {
                    amount: fee,
                    token_id: TokenId::Ziesha,
                },
                new_nonce,
            );
            wallet.add_rsend(tx.clone());
            wallet.save(wallet_path).unwrap();
            println!("Delegate-Id: {}", delegate_id);
            println!("{:#?}", client.transact(tx).await?);
            Ok::<(), NodeError>(())
        },
        req_loop
    )
    .unwrap();
}
