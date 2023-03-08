use tokio::try_join;

use crate::cli::BazukaConfig;
use crate::client::NodeError;
use crate::config;
use crate::core::MpnAddress;
use crate::wallet::{TxBuilder, Wallet};
use crate::{client::BazukaClient, core::TokenId};
use colored::Colorize;
use std::collections::HashMap;

pub async fn info(conf: Option<BazukaConfig>, wallet: Option<Wallet>) -> () {
    let mpn_log4_account_capacity = config::blockchain::get_blockchain_config()
        .mpn_config
        .log4_tree_size;
    let (conf, wallet) = conf.zip(wallet).expect("Bazuka is not initialized!");
    let tx_builder = TxBuilder::new(&wallet.seed());

    let (req_loop, client) = BazukaClient::connect(
        tx_builder.get_priv_key(),
        conf.random_node(),
        conf.network,
        None,
    );
    try_join!(
        async move {
            let acc = client.get_account(tx_builder.get_address()).await;
            let mut token_balances = HashMap::new();
            let mut tokens = HashMap::new();
            let mut token_indices = HashMap::new();
            for (i, tkn) in wallet.get_tokens().iter().enumerate() {
                token_indices.insert(*tkn, i);
                if let Ok(inf) = client.get_balance(tx_builder.get_address(), *tkn).await {
                    token_balances.insert(*tkn, inf);
                }
                if let Ok(inf) = client.get_token(*tkn).await {
                    tokens.insert(*tkn, inf);
                }
            }

            let curr_nonce = wallet.new_r_nonce().map(|n| n - 1);
            println!();
            println!("{}", "Main-chain\n---------".bright_green());
            println!(
                "{}\t{}",
                "VRF public-key:".bright_yellow(),
                tx_builder.get_vrf_public_key()
            );
            println!(
                "{}\t{}",
                "Address:".bright_yellow(),
                tx_builder.get_address()
            );
            if let Ok(resp) = acc {
                for (i, id) in wallet.get_tokens().iter().enumerate() {
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
                    if nonce > resp.account.nonce {
                        println!("(Pending transactions: {})", nonce - resp.account.nonce);
                    }
                }
            } else {
                println!("{} {}", "Error:".bright_red(), "Node not available!");
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
                    .await
                    .map(|resp| resp.account);
                if let Ok(resp) = resp {
                    let curr_z_nonce = wallet.new_z_nonce(&addr);
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
                } else {
                    println!("{} {}", "Error:".bright_red(), "Node not available!");
                }
                println!();
            }
            Ok::<(), NodeError>(())
        },
        req_loop
    )
    .unwrap();
}
