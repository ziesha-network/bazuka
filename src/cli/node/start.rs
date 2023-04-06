use crate::cli::{run_node, BazukaConfig};

use bazuka::{
    blockchain::KvStoreChain, client::messages::SocialProfiles, config, db::LevelDbKvStore,
    db::RamKvStore, wallet::WalletCollection,
};

pub async fn start(
    discord_handle: Option<String>,
    client_only: bool,
    conf: BazukaConfig,
    wallet: WalletCollection,
    dev: bool,
) {
    if dev {
        run_node(
            KvStoreChain::new(
                RamKvStore::new(),
                config::blockchain::get_dev_blockchain_config(),
            )
            .unwrap(),
            conf.clone(),
            wallet.clone(),
            SocialProfiles {
                discord: discord_handle,
            },
            client_only,
        )
        .await
        .unwrap();
    } else {
        run_node(
            KvStoreChain::new(
                LevelDbKvStore::new(&conf.db, 64).unwrap(),
                config::blockchain::get_blockchain_config(),
            )
            .unwrap(),
            conf.clone(),
            wallet.clone(),
            SocialProfiles {
                discord: discord_handle,
            },
            client_only,
        )
        .await
        .unwrap();
    }
}
