pub mod add_token;
pub mod new_token;
pub mod send;
pub mod reset;
pub mod register_validator;
pub mod delegate;
pub mod info;
pub mod resend_pending;

use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
};

pub use add_token::*;
pub use new_token::*;
pub use send::*;
pub use reset::*;
pub use register_validator::*;
pub use delegate::*;
pub use info::*;
pub use resend_pending::*;
use serde::{Deserialize, Serialize};

use crate::{client::PeerAddress, wallet::Wallet};

#[cfg(feature = "client")]
const DEFAULT_PORT: u16 = 8765;

#[cfg(feature = "client")]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BazukaConfig {
    listen: SocketAddr,
    external: PeerAddress,
    network: String,
    miner_token: String,
    bootstrap: Vec<PeerAddress>,
    db: PathBuf,
}

#[cfg(feature = "client")]
impl BazukaConfig {
    fn random_node(&self) -> PeerAddress {
        PeerAddress(SocketAddr::from(([0, 0, 0, 0], DEFAULT_PORT)))
        /*self.bootstrap
        .choose(&mut rand::thread_rng())
        .unwrap_or(&PeerAddress(SocketAddr::from(([0, 0, 0, 0], DEFAULT_PORT))))
        .clone()*/
    }
}

pub fn get_wallet() -> Option<Wallet> {
    let wallet_path = get_wallet_path();
    let wallet = Wallet::open(wallet_path.clone()).unwrap();
    wallet
}

pub fn get_wallet_path() -> PathBuf {
    let wallet_path = home::home_dir().unwrap().join(Path::new(".bazuka-wallet"));
    wallet_path
}

pub fn get_conf() -> Option<BazukaConfig> {
    let conf_path = home::home_dir().unwrap().join(Path::new(".bazuka.yaml"));
    let conf: Option<BazukaConfig> = std::fs::File::open(conf_path.clone())
        .ok()
        .map(|f| serde_yaml::from_reader(f).unwrap());
    conf
}
