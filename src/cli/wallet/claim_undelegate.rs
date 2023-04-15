use std::path::PathBuf;
use tokio::try_join;

use crate::cli::{BazukaConfig, CURRENT_NETWORK};
use bazuka::client::{BazukaClient, Limit, NodeError};
use bazuka::common::*;
use bazuka::core::{Decimal, Money, NonceGroup, TokenId, UndelegationId};
use bazuka::wallet::WalletCollection;

pub async fn claim_undelegate(
    conf: BazukaConfig,
    mut wallet: WalletCollection,
    wallet_path: &PathBuf,
    memo: Option<String>,
    undelegation_id: UndelegationId,
    fee: Decimal,
) -> () {
    let tx_builder = wallet.user(0).tx_builder();
    let (req_loop, client) = BazukaClient::connect(
        tx_builder.get_priv_key(),
        conf.random_node(),
        CURRENT_NETWORK.into(),
        Some(Limit::default().time(2 * SECOND)),
    );
    try_join!(
        async move {
            let curr_nonce = client.get_account(tx_builder.get_address()).await?.nonce;

            let new_nonce = wallet
                .user(0)
                .new_nonce(NonceGroup::TransactionAndDelta(tx_builder.get_address()))
                .unwrap_or(curr_nonce + 1);
            let tx = tx_builder.claim_undelegate(
                memo.unwrap_or_default(),
                undelegation_id,
                Money {
                    amount: fee.to_amount(bazuka::config::UNIT_ZEROS),
                    token_id: TokenId::Ziesha,
                },
                new_nonce,
            );

            if let Some(err) = client.transact(tx.clone().into()).await?.error {
                println!("Error: {}", err);
            } else {
                wallet.user(0).add_tx(tx.clone().into());
                wallet.save(wallet_path).unwrap();
                println!("Sent");
            }
            Ok::<(), NodeError>(())
        },
        req_loop
    )
    .unwrap();
}
