use super::{BazukaConfig, DEFAULT_PORT};
use crate::{client::PeerAddress, wallet::WalletCollection};
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
    conf: Option<BazukaConfig>,
    conf_path: &PathBuf,
    wallet: Option<WalletCollection>,
    wallet_path: &PathBuf,
) -> () {
    if wallet.is_none() {
        let w = WalletCollection::create(&mut rand_mnemonic::thread_rng(), mnemonic);
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
        let public_ip = crate::client::utils::get_public_ip().await.unwrap();
        std::fs::write(
            conf_path,
            serde_yaml::to_string(&BazukaConfig {
                network,
                bootstrap,
                listen: listen.unwrap_or_else(|| SocketAddr::from(([0, 0, 0, 0], DEFAULT_PORT))),
                external: external
                    .unwrap_or_else(|| PeerAddress(SocketAddr::from((public_ip, DEFAULT_PORT)))),
                db: db.unwrap_or_else(|| home::home_dir().unwrap().join(Path::new(".bazuka"))),
                mpn_workers: vec![],
            })
            .unwrap(),
        )
        .unwrap();
    } else {
        println!("Bazuka is already initialized!");
    }
}
