use tokio::try_join;

use crate::cli::BazukaConfig;
use crate::client::NodeError;
use crate::config;
use crate::core::MpnAddress;
use crate::wallet::WalletCollection;
use crate::{client::BazukaClient, core::TokenId};
use colored::Colorize;
use std::collections::HashMap;

pub async fn info(conf: Option<BazukaConfig>, wallet: Option<WalletCollection>) -> () {
    let mpn_log4_account_capacity = config::blockchain::get_blockchain_config()
        .mpn_config
        .log4_tree_size;
    let (conf, mut wallet) = conf.zip(wallet).expect("Bazuka is not initialized!");
    let val_tx_builder = wallet.validator_builder();
    let tx_builder = wallet.user_builder(0);

    let (req_loop, client) =
        BazukaClient::connect(tx_builder.get_priv_key(), conf.random_node(), conf.network);
    try_join!(
        async move {
            println!();
            println!("{}", "Validator Info\n---------".bright_green());
            println!(
                "{}\t{}",
                "Validator address:".bright_yellow(),
                val_tx_builder.get_address()
            );

            let validator_ziesha = client
                .get_balance(val_tx_builder.get_address(), TokenId::Ziesha)
                .await?
                .balance;
            println!(
                "{}\t{}{}",
                "Validator main-chain balance:".bright_yellow(),
                validator_ziesha.display_by_decimals(crate::config::UNIT_ZEROS),
                crate::config::SYMBOL
            );

            let validator_mpn_ziesha = client
                .get_mpn_account(
                    val_tx_builder
                        .get_mpn_address()
                        .account_index(mpn_log4_account_capacity),
                )
                .await?
                .account
                .tokens
                .get(&0)
                .map(|tkn| {
                    if tkn.token_id == TokenId::Ziesha {
                        tkn.amount
                    } else {
                        0.into()
                    }
                })
                .unwrap_or_default();
            println!(
                "{}\t{}{}",
                "Validator MPN balance:".bright_yellow(),
                validator_mpn_ziesha.display_by_decimals(crate::config::UNIT_ZEROS),
                crate::config::SYMBOL
            );

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
                        amount.display_by_decimals(crate::config::UNIT_ZEROS),
                        crate::config::SYMBOL
                    );
                }
            }

            if !delegations.delegatees.is_empty() {
                println!();
                println!("{}", "Delegatees\n---------".bright_green());
                for (addr, amount) in delegations.delegatees.iter() {
                    println!(
                        "You -> {} ({}{})",
                        addr,
                        amount.display_by_decimals(crate::config::UNIT_ZEROS),
                        crate::config::SYMBOL
                    );
                }
            }

            let acc = client.get_account(tx_builder.get_address()).await?;
            let mut token_balances = HashMap::new();
            let mut tokens = HashMap::new();
            let mut token_indices = HashMap::new();
            for (i, tkn) in wallet.user(0).get_tokens().iter().enumerate() {
                token_indices.insert(*tkn, i);
                token_balances.insert(
                    *tkn,
                    client.get_balance(tx_builder.get_address(), *tkn).await?,
                );
                tokens.insert(*tkn, client.get_token(*tkn).await?);
            }

            let curr_nonce = wallet.user(0).new_r_nonce().map(|n| n - 1);

            println!();
            println!("{}", "Main-chain\n---------".bright_green());
            println!(
                "{}\t{}",
                "Address:".bright_yellow(),
                tx_builder.get_address()
            );
            for (i, id) in wallet.user(0).get_tokens().iter().enumerate() {
                if let Some(inf) = token_balances.get(id) {
                    println!(
                        "{}\t{}{}",
                        format!("#{} <{}>:", i, inf.name).bright_yellow(),
                        inf.balance
                            .display_by_decimals(tokens.get(id).unwrap().token.decimals),
                        if *id == TokenId::Ziesha {
                            crate::config::SYMBOL.to_string()
                        } else {
                            format!(" {} (Token-Id: {})", inf.symbol, id)
                        }
                    );
                } else {
                    println!("{}\t{}", format!("#{}:", i).bright_yellow(), "N/A");
                }
            }
            if let Some(nonce) = curr_nonce {
                if nonce > acc.account.nonce {
                    println!("(Pending transactions: {})", nonce - acc.account.nonce);
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
                let resp = client
                    .get_mpn_account(addr.account_index(mpn_log4_account_capacity))
                    .await?
                    .account;
                let curr_z_nonce = wallet.user(0).new_z_nonce(&addr);
                if !resp.address.is_on_curve() {
                    println!(
                        "{}\t{}",
                        "Address:".bright_yellow(),
                        MpnAddress {
                            pub_key: tx_builder.get_zk_address(),
                        }
                    );
                    println!("Waiting to be activated... (Send some funds to it!)")
                } else {
                    let acc_pk = crate::crypto::jubjub::PublicKey(resp.address.compress());
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
                            let token_index = token_indices[&money.token_id];
                            println!(
                                "{}\t{}{}",
                                format!("#{} <{}>:", token_index, inf.name).bright_yellow(),
                                money.amount.display_by_decimals(resp.token.decimals),
                                if money.token_id == TokenId::Ziesha {
                                    crate::config::SYMBOL.to_string()
                                } else {
                                    format!(" {}", inf.symbol)
                                }
                            );
                        }
                    }
                }
                if let Some(nonce) = curr_z_nonce {
                    if nonce > resp.nonce {
                        println!("(Pending transactions: {})", nonce - resp.nonce);
                    }
                }
                println!();
            }
            Ok::<(), NodeError>(())
        },
        req_loop
    )
    .unwrap();
}
