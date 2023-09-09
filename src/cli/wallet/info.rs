use tokio::try_join;

use crate::cli::{BazukaConfig, CURRENT_NETWORK};
use bazuka::client::{Limit, NodeError};
use bazuka::common::*;
use bazuka::core::{MpnAddress, NonceGroup};
use bazuka::wallet::WalletCollection;
use bazuka::{client::BazukaClient, core::ContractId};
use colored::Colorize;
use std::collections::HashMap;

pub async fn info(conf: BazukaConfig, mut wallet: WalletCollection, validator: bool) -> () {
    let val_tx_builder = wallet.validator().tx_builder();
    let tx_builder = wallet.user(0).tx_builder();

    let (req_loop, client) = BazukaClient::connect(
        tx_builder.get_priv_key(),
        conf.random_node(),
        CURRENT_NETWORK.into(),
        Some(Limit::default().time(2 * SECOND)),
    );
    try_join!(
        async move {
            if validator {
                let acc = client.get_account(val_tx_builder.get_address()).await?;

                println!("{}", "Validator Info\n---------".bright_green());
                println!(
                    "{}\t{}",
                    "Validator address:".bright_yellow(),
                    val_tx_builder.get_address()
                );

                let validator_ziesha = client
                    .get_balance(val_tx_builder.get_address(), ContractId::Ziesha)
                    .await?
                    .balance;
                println!(
                    "{}\t{}{}",
                    "Validator main-chain balance:".bright_yellow(),
                    validator_ziesha.display_by_decimals(bazuka::config::UNIT_ZEROS),
                    bazuka::config::SYMBOL
                );
                println!(
                    "{}\t{}",
                    "Validator VRF:".bright_yellow(),
                    val_tx_builder.get_vrf_public_key()
                );

                let validator_mpn_ziesha = client
                    .get_mpn_account(val_tx_builder.get_mpn_address())
                    .await?
                    .account
                    .tokens
                    .get(&0)
                    .map(|tkn| {
                        if tkn.token_id == ContractId::Ziesha {
                            tkn.amount
                        } else {
                            0.into()
                        }
                    })
                    .unwrap_or_default();
                println!(
                    "{}\t{}{}",
                    "Validator MPN balance:".bright_yellow(),
                    validator_mpn_ziesha.display_by_decimals(bazuka::config::UNIT_ZEROS),
                    bazuka::config::SYMBOL
                );
                let curr_nonce = wallet
                    .validator()
                    .new_nonce(NonceGroup::TransactionAndDelta(
                        val_tx_builder.get_address(),
                    ))
                    .map(|n| n - 1);
                if let Some(nonce) = curr_nonce {
                    if nonce > acc.nonce {
                        println!("(Pending transactions: {})", nonce - acc.nonce);
                    }
                }

                let delegations = client
                    .get_delegations(val_tx_builder.get_address(), 100)
                    .await?;

                if !delegations.delegators.is_empty() {
                    println!();
                    println!("{}", "Delegators\n---------".bright_green());
                    for (addr, amount) in delegations.delegators.iter() {
                        println!(
                            "{} -> You ({}{})",
                            addr,
                            amount.display_by_decimals(bazuka::config::UNIT_ZEROS),
                            bazuka::config::SYMBOL
                        );
                    }
                }
            } else {
                let height = client.stats().await?.height;
                let acc = client.get_account(tx_builder.get_address()).await?;
                let mut token_balances = HashMap::new();
                let mut tokens = HashMap::new();
                for tkn in wallet.user(0).get_tokens().iter() {
                    if let Some(inf) = client.get_token(*tkn).await?.token {
                        token_balances.insert(
                            *tkn,
                            client.get_balance(tx_builder.get_address(), *tkn).await?,
                        );
                        tokens.insert(*tkn, inf);
                    }
                }

                let curr_nonce = wallet
                    .user(0)
                    .new_nonce(NonceGroup::TransactionAndDelta(tx_builder.get_address()))
                    .map(|n| n - 1);
                let curr_mpn_deposit_nonce = wallet
                    .user(0)
                    .new_nonce(NonceGroup::MpnDeposit(tx_builder.get_address()))
                    .map(|n| n - 1);

                println!();
                println!("{}", "Main-chain\n---------".bright_green());
                println!(
                    "{}\t{}",
                    "Address:".bright_yellow(),
                    tx_builder.get_address()
                );
                for id in wallet.user(0).get_tokens().iter() {
                    if let Some(inf) = token_balances.get(id) {
                        println!(
                            "{}\t{}{}",
                            format!("<{}>:", inf.name).bright_yellow(),
                            inf.balance
                                .display_by_decimals(tokens.get(id).unwrap().decimals),
                            if *id == ContractId::Ziesha {
                                bazuka::config::SYMBOL.to_string()
                            } else {
                                format!(" {} (Token-Id: {})", inf.symbol, id)
                            }
                        );
                    }
                }
                if let Some(nonce) = curr_nonce {
                    if nonce > acc.nonce {
                        println!("(Pending transactions: {})", nonce - acc.nonce);
                    }
                }
                if let Some(nonce) = curr_mpn_deposit_nonce {
                    if nonce > acc.mpn_deposit_nonce {
                        println!("(Pending deposits: {})", nonce - acc.mpn_deposit_nonce);
                    }
                }

                let delegations = client
                    .get_delegations(tx_builder.get_address(), 100)
                    .await?;

                if !delegations.delegatees.is_empty() {
                    println!();
                    println!("{}", "Delegatees\n---------".bright_green());
                    for (addr, amount) in delegations.delegatees.iter() {
                        println!(
                            "You -> {} ({}{})",
                            addr,
                            amount.display_by_decimals(bazuka::config::UNIT_ZEROS),
                            bazuka::config::SYMBOL
                        );
                    }
                }

                if !delegations.undelegations.is_empty() {
                    println!();
                    println!("{}", "Undelegations\n---------".bright_green());
                    for (_, undel) in delegations.undelegations {
                        println!(
                            "Receiving {}{} after {} blocks...",
                            undel.amount.display_by_decimals(bazuka::config::UNIT_ZEROS),
                            bazuka::config::SYMBOL,
                            undel.unlocks_on - height
                        );
                    }
                }

                println!();

                let mpn_address = MpnAddress {
                    pub_key: tx_builder.get_zk_address(),
                };

                for (i, addr) in [mpn_address].into_iter().enumerate() {
                    println!(
                        "{}",
                        format!("MPN Account #{}\n---------", i).bright_green()
                    );
                    let resp = client.get_mpn_account(addr.clone()).await?.account;
                    let curr_mpn_tx_nonce = wallet
                        .user(0)
                        .new_nonce(NonceGroup::MpnTransaction(addr.clone()));
                    let curr_mpn_withdraw_nonce =
                        wallet.user(0).new_nonce(NonceGroup::MpnWithdraw(addr));
                    if !resp.address.is_on_curve() {
                        println!(
                            "{}\t{}",
                            "Address:".bright_yellow(),
                            MpnAddress {
                                pub_key: tx_builder.get_zk_address(),
                            }
                        );
                        println!("No tokens in your wallet yet!")
                    } else {
                        let acc_pk = bazuka::crypto::jubjub::PublicKey(resp.address.compress());
                        if acc_pk != tx_builder.get_zk_address() {
                            println!(
                                "{} {}",
                                "Error:".bright_red(),
                                "Slot acquired by someone else!"
                            );
                            continue;
                        }
                        println!(
                            "{}\t{}",
                            "Address:".bright_yellow(),
                            MpnAddress { pub_key: acc_pk }
                        );
                        for (_, money) in resp.tokens.iter() {
                            let resp = client
                                .get_token(money.token_id)
                                .await
                                .map(|resp| resp)
                                .unwrap();
                            if let Some(inf) = token_balances.get(&money.token_id) {
                                println!(
                                    "{}\t{}{}",
                                    format!("<{}>:", inf.name).bright_yellow(),
                                    resp.token
                                        .as_ref()
                                        .map(|t| money.amount.display_by_decimals(t.decimals))
                                        .unwrap_or("N/A".to_string()),
                                    if money.token_id == ContractId::Ziesha {
                                        bazuka::config::SYMBOL.to_string()
                                    } else {
                                        format!(" {}", inf.symbol)
                                    }
                                );
                            }
                        }
                    }
                    if let Some(nonce) = curr_mpn_tx_nonce {
                        if nonce > resp.tx_nonce + 1 {
                            println!("(Pending transactions: {})", nonce - resp.tx_nonce - 1);
                        }
                    }
                    if let Some(nonce) = curr_mpn_withdraw_nonce {
                        if nonce > resp.withdraw_nonce + 1 {
                            println!("(Pending withdrawals: {})", nonce - resp.withdraw_nonce - 1);
                        }
                    }
                    println!();
                }
            }
            Ok::<(), NodeError>(())
        },
        req_loop
    )
    .unwrap();
}
