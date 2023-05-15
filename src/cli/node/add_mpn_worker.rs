use std::path::PathBuf;

use crate::cli::{BazukaConfig, BazukaConfigMpnWorker};
use bazuka::core::Address;

pub async fn add_mpn_worker(conf_path: &PathBuf, conf: BazukaConfig, address: Address) {
    let mut conf = conf.clone();
    conf.mpn_workers.push(BazukaConfigMpnWorker {
        address: address.to_string(),
    });
    std::fs::write(conf_path, serde_yaml::to_string(&conf).unwrap()).unwrap();
}
