use crate::{
    cli::{run_node, BazukaConfig},
    client::messages::SocialProfiles,
    wallet::Wallet,
};

pub async fn start(
    discord_handle: Option<String>,
    client_only: bool,
    conf: &BazukaConfig,
    wallet: &Wallet,
) {
    run_node(
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
