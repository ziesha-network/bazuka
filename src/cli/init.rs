use super::{
    generate_miner_token, get_conf, get_conf_path, get_wallet, get_wallet_path, BazukaConfig,
    DEFAULT_PORT,
};
use crate::{client::PeerAddress, wallet::Wallet};
use bip39::Mnemonic;
use colored::Colorize;
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
};

pub async fn init(
    network: String,
    bootstrap: Vec<PeerAddress>,
    mnemonic: Option<Mnemonic>,
    external: Option<PeerAddress>,
    listen: Option<SocketAddr>,
    db: Option<PathBuf>,
) -> () {
    let wallet = get_wallet();
    let wallet_path = get_wallet_path();
    let conf_path = get_conf_path();
    let conf = get_conf();
    if wallet.is_none() {
        let w = Wallet::create(&mut rand_mnemonic::thread_rng(), mnemonic);
        w.save(wallet_path).unwrap();
        println!("Wallet generated!");
        println!("{} {}", "Mnemonic phrase:".bright_yellow(), w.mnemonic());
        println!(
            "{}",
            "WRITE DOWN YOUR MNEMONIC PHRASE IN A SAFE PLACE!"
                .italic()
                .bold()
                .bright_green()
        );
    } else {
        println!("Wallet is already initialized!");
    }

    if conf.is_none() {
        let miner_token = generate_miner_token();
        let public_ip = crate::client::utils::get_public_ip().await.unwrap();
        std::fs::write(
            conf_path,
            serde_yaml::to_string(&BazukaConfig {
                network,
                miner_token,
                bootstrap,
                listen: listen.unwrap_or_else(|| SocketAddr::from(([0, 0, 0, 0], DEFAULT_PORT))),
                external: external
                    .unwrap_or_else(|| PeerAddress(SocketAddr::from((public_ip, DEFAULT_PORT)))),
                db: db.unwrap_or_else(|| home::home_dir().unwrap().join(Path::new(".bazuka"))),
            })
            .unwrap(),
        )
        .unwrap();
    } else {
        println!("Bazuka is already initialized!");
    }
}
