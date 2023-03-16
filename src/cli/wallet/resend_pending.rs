use crate::cli::{get_conf, get_wallet_collection, get_wallet_path, BazukaConfig};
use crate::client::BazukaClient;
use crate::client::NodeError;
use crate::config::blockchain;
use crate::core::MpnAddress;
use crate::core::{Amount, MpnSourcedTx};
use crate::core::{ChainSourcedTx, Money, TokenId};
use crate::wallet::WalletCollection;
use tokio::try_join;

#[cfg(feature = "client")]
#[allow(dead_code)]
async fn resend_all_wallet_txs(
    conf: BazukaConfig,
    wallet: &mut WalletCollection,
    fill_gaps: bool,
    shift: bool,
) -> Result<(), NodeError> {
    let tx_builder = wallet.user_builder(0);
    let (req_loop, client) =
        BazukaClient::connect(tx_builder.get_priv_key(), conf.random_node(), conf.network);
    let mpn_log4_account_capacity = blockchain::get_blockchain_config()
        .mpn_config
        .log4_tree_size;
    try_join!(
        async move {
            let curr_nonce = client
                .get_account(tx_builder.get_address())
                .await?
                .account
                .nonce;
            let mut curr_mpn_nonce = client
                .get_mpn_account(
                    MpnAddress {
                        pub_key: tx_builder.get_zk_address(),
                    }
                    .account_index(mpn_log4_account_capacity),
                )
                .await?
                .account
                .nonce;
            if shift {
                wallet
                    .user(0)
                    .delete_chain_tx(curr_nonce + 1, tx_builder.clone());
            }
            for tx in wallet.user(0).chain_sourced_txs.iter() {
                if tx.nonce() >= curr_nonce {
                    match tx {
                        ChainSourcedTx::TransactionAndDelta(tx) => {
                            client.transact(tx.clone()).await?;
                        }
                        ChainSourcedTx::MpnDeposit(tx) => {
                            client.transact_contract_deposit(tx.clone()).await?;
                        }
                    }
                }
            }
            for acc in wallet.user(0).mpn_sourced_txs.values() {
                for tx in acc.iter() {
                    if tx.nonce() >= curr_mpn_nonce {
                        if fill_gaps {
                            while curr_mpn_nonce != tx.nonce() {
                                let filler = tx_builder.create_mpn_transaction(
                                    0,
                                    MpnAddress {
                                        pub_key: tx_builder.get_zk_address(),
                                    },
                                    0,
                                    Money {
                                        amount: Amount(0),
                                        token_id: TokenId::Ziesha,
                                    },
                                    0,
                                    Money {
                                        amount: Amount(0),
                                        token_id: TokenId::Ziesha,
                                    },
                                    curr_mpn_nonce,
                                );
                                client.zero_transact(filler).await?;
                                curr_mpn_nonce += 1;
                            }
                        }
                        match tx {
                            MpnSourcedTx::MpnTransaction(tx) => {
                                let tkn = client.get_token(tx.amount.token_id).await?;
                                println!(
                                    "Send {} from MPN-account to {} (Nonce: {})",
                                    tx.amount.amount.display_by_decimals(tkn.token.decimals),
                                    tx.dst_pub_key,
                                    tx.nonce
                                );
                                client.zero_transact(tx.clone()).await?;
                            }
                            MpnSourcedTx::MpnWithdraw(tx) => {
                                let tkn = client.get_token(tx.payment.amount.token_id).await?;
                                println!(
                                    "Send {} from MPN-account to {} (Nonce: {})",
                                    tx.payment
                                        .amount
                                        .amount
                                        .display_by_decimals(tkn.token.decimals),
                                    tx.payment.dst,
                                    tx.zk_nonce
                                );
                                client.transact_contract_withdraw(tx.clone()).await?;
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

    Ok(())
}

pub async fn resend_pending(fill_gaps: bool, shift: bool) -> () {
    let wallet = get_wallet_collection();
    let wallet_path = get_wallet_path();
    let conf = get_conf();
    let (conf, mut wallet) = conf.zip(wallet).expect("Bazuka is not initialized!");
    resend_all_wallet_txs(conf, &mut wallet, fill_gaps, shift)
        .await
        .unwrap();
    wallet.save(wallet_path).unwrap();
}
