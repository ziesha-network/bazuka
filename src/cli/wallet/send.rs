use std::path::PathBuf;

use crate::cli::BazukaConfig;
use crate::wallet::WalletCollection;
use crate::{
    client::{messages::TransactRequest, BazukaClient, NodeError},
    config,
    core::{Amount, GeneralAddress, GeneralTransaction, Money, TokenId},
};
use tokio::try_join;

pub async fn send(
    memo: Option<String>,
    from: GeneralAddress,
    to: GeneralAddress,
    amount: Amount,
    fee: Amount,
    token: Option<usize>,
    conf: Option<BazukaConfig>,
    wallet: Option<WalletCollection>,
    wallet_path: &PathBuf,
) {
    let (conf, mut wallet) = conf.zip(wallet).expect("Bazuka is not initialized!");
    let tx_builder = wallet.user_builder(0);
    let log4_token_tree_size = config::blockchain::get_blockchain_config()
        .mpn_config
        .log4_token_tree_size;
    let mpn_log4_account_capacity = config::blockchain::get_blockchain_config()
        .mpn_config
        .log4_tree_size;
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
                            let curr_nonce = client
                                .get_account(tx_builder.get_address())
                                .await?
                                .account
                                .nonce;
                            let new_nonce = wallet.user(0).new_nonce().unwrap_or(curr_nonce + 1);
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
                                .account
                                .nonce;
                            let dst_acc = client
                                .get_mpn_account(to.account_index(mpn_log4_account_capacity))
                                .await?
                                .account;
                            let to_token_index = if let Some(ind) =
                                dst_acc.find_token_index(log4_token_tree_size, tkn, true)
                            {
                                ind
                            } else {
                                panic!("Cannot find empty token slot in your MPN account!");
                            };
                            let new_nonce = wallet.user(0).new_nonce().unwrap_or(curr_nonce + 1);
                            let pay = tx_builder.deposit_mpn(
                                memo.unwrap_or_default(),
                                mpn_contract_id,
                                to,
                                to_token_index,
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
                            let acc = client
                                .get_mpn_account(from.account_index(mpn_log4_account_capacity))
                                .await?
                                .account;
                            let token_index = if let Some(ind) =
                                acc.find_token_index(log4_token_tree_size, tkn, false)
                            {
                                ind
                            } else {
                                panic!("Token not found in your account!");
                            };
                            let fee_token_index = if let Some(ind) =
                                acc.find_token_index(log4_token_tree_size, TokenId::Ziesha, false)
                            {
                                ind
                            } else {
                                panic!("Token not found in your account!");
                            };
                            let new_nonce = wallet.user(0).new_nonce(&from).unwrap_or(acc.nonce);
                            let pay = tx_builder.withdraw_mpn(
                                memo.unwrap_or_default(),
                                mpn_contract_id,
                                new_nonce,
                                token_index,
                                Money {
                                    amount,
                                    token_id: tkn,
                                },
                                fee_token_index,
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
                            let acc = client
                                .get_mpn_account(from.account_index(mpn_log4_account_capacity))
                                .await?
                                .account;
                            let dst_acc = client
                                .get_mpn_account(to.account_index(mpn_log4_account_capacity))
                                .await?
                                .account;
                            let to_token_index = if let Some(ind) =
                                dst_acc.find_token_index(log4_token_tree_size, tkn, true)
                            {
                                ind
                            } else {
                                panic!("Token not found in your account!");
                            };
                            let token_index = if let Some(ind) =
                                acc.find_token_index(log4_token_tree_size, tkn, false)
                            {
                                ind
                            } else {
                                panic!("Token not found in your account!");
                            };
                            let fee_token_index = if let Some(ind) =
                                acc.find_token_index(log4_token_tree_size, TokenId::Ziesha, false)
                            {
                                ind
                            } else {
                                panic!("Token not found in your account!");
                            };
                            let new_nonce = wallet.user(0).new_nonce(&from).unwrap_or(acc.nonce);
                            let tx = tx_builder.create_mpn_transaction(
                                token_index,
                                to,
                                to_token_index,
                                Money {
                                    amount,
                                    token_id: tkn,
                                },
                                fee_token_index,
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
