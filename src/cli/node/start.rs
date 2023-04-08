use crate::cli::{run_node, BazukaConfig, CURRENT_NETWORK};

use bazuka::{
    blockchain::KvStoreChain, client::messages::SocialProfiles, config, db::LevelDbKvStore,
    db::RamKvStore, wallet::WalletCollection,
};

pub async fn start(
    discord_handle: Option<String>,
    client_only: bool,
    conf: BazukaConfig,
    mut wallet: WalletCollection,
    ram: bool,
    dev: bool,
) {
    let blockchain_conf = if dev {
        let validator_wallet = wallet.validator().tx_builder();
        config::blockchain::get_dev_blockchain_config(&validator_wallet)
    } else {
        config::blockchain::get_blockchain_config()
    };

    if ram {
        run_node(
            KvStoreChain::new(RamKvStore::new(), blockchain_conf).unwrap(),
            conf.clone(),
            wallet.clone(),
            SocialProfiles {
                discord: discord_handle,
            },
            client_only,
            "dev".into(),
        )
        .await
        .unwrap();
    } else {
        run_node(
            KvStoreChain::new(LevelDbKvStore::new(&conf.db, 64).unwrap(), blockchain_conf).unwrap(),
            conf.clone(),
            wallet.clone(),
            SocialProfiles {
                discord: discord_handle,
            },
            client_only,
            CURRENT_NETWORK.into(),
        )
        .await
        .unwrap();
    }
}
