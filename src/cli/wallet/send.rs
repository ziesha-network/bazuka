use std::path::PathBuf;

use crate::cli::BazukaConfig;
use bazuka::wallet::WalletCollection;
use bazuka::{
    client::{BazukaClient, NodeError},
    config,
    core::{Amount, GeneralAddress, Money, NonceGroup, TokenId},
};
use tokio::try_join;

pub async fn send(
    memo: Option<String>,
    from: GeneralAddress,
    to: GeneralAddress,
    amount: Amount,
    fee: Amount,
    token: Option<usize>,
    conf: BazukaConfig,
    mut wallet: WalletCollection,
    wallet_path: &PathBuf,
) {
    let tx_builder = wallet.user(0).tx_builder();
    let mpn_contract_id = config::blockchain::get_blockchain_config()
        .mpn_config
        .mpn_contract_id;

    let (req_loop, client) =
        BazukaClient::connect(tx_builder.get_priv_key(), conf.random_node(), conf.network);
    let tkn = if let Some(token) = token {
        if token >= wallet.user(0).get_tokens().len() {
            panic!("Wrong token selected!");
        } else {
            wallet.user(0).get_tokens()[token]
        }
    } else {
        TokenId::Ziesha
    };
    match from {
        GeneralAddress::ChainAddress(from) => {
            if tx_builder.get_address() != from {
                panic!("Source address doesn't exist in your wallet!");
            }
            match to {
                GeneralAddress::ChainAddress(to) => {
                    try_join!(
                        async move {
                            let curr_nonce =
                                client.get_account(tx_builder.get_address()).await?.nonce;
                            let new_nonce = wallet
                                .user(0)
                                .new_nonce(NonceGroup::TransactionAndDelta(
                                    tx_builder.get_address(),
                                ))
                                .unwrap_or(curr_nonce + 1);
                            let tx = tx_builder.create_transaction(
                                memo.unwrap_or_default(),
                                to,
                                Money {
                                    amount,
                                    token_id: tkn,
                                },
                                Money {
                                    amount: fee,
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
                GeneralAddress::MpnAddress(to) => {
                    try_join!(
                        async move {
                            let curr_nonce = client
                                .get_account(tx_builder.get_address())
                                .await?
                                .mpn_deposit_nonce;
                            let new_nonce = wallet
                                .user(0)
                                .new_nonce(NonceGroup::MpnDeposit(tx_builder.get_address()))
                                .unwrap_or(curr_nonce + 1);
                            let pay = tx_builder.deposit_mpn(
                                memo.unwrap_or_default(),
                                mpn_contract_id,
                                to,
                                new_nonce,
                                Money {
                                    amount,
                                    token_id: tkn,
                                },
                                Money {
                                    amount: fee,
                                    token_id: TokenId::Ziesha,
                                },
                            );
                            wallet.user(0).add_tx(pay.clone().into());
                            wallet.save(wallet_path).unwrap();
                            println!("{:#?}", client.transact(pay.clone().into()).await?);
                            Ok::<(), NodeError>(())
                        },
                        req_loop
                    )
                    .unwrap();
                }
            }
        }
        GeneralAddress::MpnAddress(from) => {
            if tx_builder.get_zk_address() != from.pub_key {
                panic!("Source address doesn't exist in your wallet!");
            }
            match to {
                GeneralAddress::ChainAddress(to) => {
                    try_join!(
                        async move {
                            let acc = client.get_mpn_account(from.clone()).await?.account;
                            let new_nonce = wallet
                                .user(0)
                                .new_nonce(NonceGroup::MpnWithdraw(tx_builder.get_mpn_address()))
                                .unwrap_or(acc.withdraw_nonce + 1);
                            let pay = tx_builder.withdraw_mpn(
                                memo.unwrap_or_default(),
                                mpn_contract_id,
                                new_nonce,
                                Money {
                                    amount,
                                    token_id: tkn,
                                },
                                Money {
                                    amount: fee,
                                    token_id: TokenId::Ziesha,
                                },
                                to.to_string().parse().unwrap(), // TODO: WTH :D
                            );
                            wallet.user(0).add_tx(pay.clone().into());
                            wallet.save(wallet_path).unwrap();
                            println!("{:#?}", client.transact(pay.clone().into()).await?);
                            Ok::<(), NodeError>(())
                        },
                        req_loop
                    )
                    .unwrap();
                }
                GeneralAddress::MpnAddress(to) => {
                    try_join!(
                        async move {
                            if memo.is_some() {
                                panic!("Cannot assign a memo to a MPN-to-MPN transaction!");
                            }
                            let acc = client.get_mpn_account(from).await?.account;
                            let new_nonce = wallet
                                .user(0)
                                .new_nonce(NonceGroup::MpnTransaction(tx_builder.get_mpn_address()))
                                .unwrap_or(acc.tx_nonce + 1);
                            let tx = tx_builder.create_mpn_transaction(
                                to,
                                Money {
                                    amount,
                                    token_id: tkn,
                                },
                                Money {
                                    amount: fee,
                                    token_id: TokenId::Ziesha,
                                },
                                new_nonce,
                            );
                            wallet.user(0).add_tx(tx.clone().into());
                            wallet.save(wallet_path).unwrap();
                            println!("{:#?}", client.transact(tx.clone().into()).await?);
                            Ok::<(), NodeError>(())
                        },
                        req_loop
                    )
                    .unwrap();
                }
            }
        }
    }
}
