use std::path::PathBuf;

use crate::cli::{BazukaConfig, CURRENT_NETWORK};
use bazuka::wallet::WalletCollection;
use bazuka::{
    client::{BazukaClient, Limit, NodeError},
    common::*,
    config,
    core::{Amount, Decimal, GeneralAddress, Money, NonceGroup, TokenId, TransactionKind},
};
use colored::Colorize;
use std::collections::HashMap;
use tokio::try_join;

pub async fn send(
    memo: Option<String>,
    from: GeneralAddress,
    to: GeneralAddress,
    amount: Decimal,
    fee: Decimal,
    token_id: Option<TokenId>,
    conf: BazukaConfig,
    mut wallet: WalletCollection,
    wallet_path: &PathBuf,
) {
    let tx_builder = wallet.user(0).tx_builder();
    let mpn_contract_id = config::blockchain::get_blockchain_config()
        .mpn_config
        .mpn_contract_id;

    let (req_loop, client) = BazukaClient::connect(
        tx_builder.get_priv_key(),
        conf.random_node(),
        CURRENT_NETWORK.into(),
        Some(Limit::default().time(2 * SECOND)),
    );
    let tkn = if let Some(token_id) = token_id {
        if !wallet.user(0).get_tokens().contains(&token_id) {
            panic!("Token does not exist in your wallet!");
        } else {
            token_id
        }
    } else {
        TokenId::Ziesha
    };

    try_join!(
        async move {
            let median_fees = client.stats().await?.median_fees;
            let fee_amount = fee.to_amount(bazuka::config::UNIT_ZEROS);
            let tkn_decimals = client
                .get_token(tkn)
                .await?
                .token
                .expect("Token not found!")
                .decimals;
            fn fee_enough(
                kind: TransactionKind,
                median_fees: &HashMap<TransactionKind, Amount>,
                fee_amount: Amount,
            ) -> bool {
                let median_fee = median_fees.get(&kind).cloned().unwrap_or_default();
                if fee_amount < median_fee {
                    println!(
                        "{} Fee is less than median fee of the network: {}{}",
                        "WARN:".yellow(),
                        median_fee.display_by_decimals(bazuka::config::UNIT_ZEROS),
                        bazuka::config::SYMBOL,
                    );
                    println!("Cancelling transaction...");
                    return false;
                }
                true
            }
            match from {
                GeneralAddress::ChainAddress(from) => {
                    if tx_builder.get_address() != from {
                        panic!("Source address doesn't exist in your wallet!");
                    }
                    match to {
                        GeneralAddress::ChainAddress(to) => {
                            if !fee_enough(
                                TransactionKind::TransactionAndDelta,
                                &median_fees,
                                fee_amount,
                            ) {
                                return Ok(());
                            }
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
                                    amount: amount.to_amount(tkn_decimals),
                                    token_id: tkn,
                                },
                                Money {
                                    amount: fee_amount,
                                    token_id: TokenId::Ziesha,
                                },
                                new_nonce,
                            );

                            if let Some(err) = client.transact(tx.clone().into()).await?.error {
                                println!("Error: {}", err);
                            } else {
                                wallet.user(0).add_tx(tx.clone().into());
                                wallet.save(wallet_path).unwrap();
                                println!("Sent!");
                            }
                        }
                        GeneralAddress::MpnAddress(to) => {
                            if !fee_enough(TransactionKind::MpnDeposit, &median_fees, fee_amount) {
                                return Ok(());
                            }
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
                                    amount: amount.to_amount(tkn_decimals),
                                    token_id: tkn,
                                },
                                Money {
                                    amount: fee_amount,
                                    token_id: TokenId::Ziesha,
                                },
                            );
                            if let Some(err) = client.transact(pay.clone().into()).await?.error {
                                println!("Error: {}", err);
                            } else {
                                wallet.user(0).add_tx(pay.clone().into());
                                wallet.save(wallet_path).unwrap();
                                println!("Sent!");
                            }
                        }
                    }
                }

                GeneralAddress::MpnAddress(from) => {
                    if tx_builder.get_zk_address() != from.pub_key {
                        panic!("Source address doesn't exist in your wallet!");
                    }
                    match to {
                        GeneralAddress::ChainAddress(to) => {
                            if !fee_enough(TransactionKind::MpnWithdraw, &median_fees, fee_amount) {
                                return Ok(());
                            }
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
                                    amount: amount.to_amount(tkn_decimals),
                                    token_id: tkn,
                                },
                                Money {
                                    amount: fee_amount,
                                    token_id: TokenId::Ziesha,
                                },
                                to.to_string().parse().unwrap(), // TODO: WTH :D
                            );

                            if let Some(err) = client.transact(pay.clone().into()).await?.error {
                                println!("Error: {}", err);
                            } else {
                                wallet.user(0).add_tx(pay.clone().into());
                                wallet.save(wallet_path).unwrap();
                                println!("Sent!");
                            }
                        }

                        GeneralAddress::MpnAddress(to) => {
                            if !fee_enough(
                                TransactionKind::MpnTransaction,
                                &median_fees,
                                fee_amount,
                            ) {
                                return Ok(());
                            }
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
                                    amount: amount.to_amount(tkn_decimals),
                                    token_id: tkn,
                                },
                                Money {
                                    amount: fee_amount,
                                    token_id: TokenId::Ziesha,
                                },
                                new_nonce,
                            );
                            if let Some(err) = client.transact(tx.clone().into()).await?.error {
                                println!("Error: {}", err);
                            } else {
                                wallet.user(0).add_tx(tx.clone().into());
                                wallet.save(wallet_path).unwrap();
                                println!("Sent!");
                            }
                        }
                    }
                }
            }
            Ok::<(), NodeError>(())
        },
        req_loop
    )
    .unwrap();
}
