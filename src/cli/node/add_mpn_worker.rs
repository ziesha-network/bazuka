use std::path::PathBuf;

use crate::{
    cli::{BazukaConfig, BazukaConfigMpnWorker},
    core::MpnAddress,
};

pub async fn add_mpn_worker(
    conf_path: &PathBuf,
    conf: Option<BazukaConfig>,
    mpn_address: MpnAddress,
) {
    let mut conf = conf.expect("Bazuka is not initialized!");
    let token = uuid::Uuid::new_v4().to_string();
    conf.mpn_workers.push(BazukaConfigMpnWorker {
        token,
        mpn_address: mpn_address.to_string(),
    });
    std::fs::write(conf_path, serde_yaml::to_string(&conf).unwrap()).unwrap();
}
