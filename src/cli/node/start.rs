use crate::{
    cli::{get_conf, get_wallet, run_node},
    client::messages::SocialProfiles,
};

pub async fn start(discord_handle: Option<String>, client_only: bool) {
    let conf = get_conf().expect("Bazuka is not initialized!");
    let wallet = get_wallet().expect("Wallet is not initialized!");
    run_node(
        conf.clone(),
        wallet.clone(),
        SocialProfiles {
            discord: discord_handle,
        },
        client_only,
    )
    .await;
}
