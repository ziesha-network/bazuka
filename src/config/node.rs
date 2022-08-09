use crate::node::NodeOptions;
use std::time::Duration;

pub fn get_mainnet_options() -> NodeOptions {
    NodeOptions {
        heartbeat_interval: Duration::from_secs(1),
        num_peers: 8,
        default_punish: 10,
        no_response_punish: 60,
        invalid_data_punish: 120,
        incorrect_power_punish: 120,
        max_punish: 600,
        outdated_heights_threshold: 15,
        state_unavailable_ban_time: 30,
        ip_request_limit_per_minute: 60,
        network: "mainnet".into(),
    }
}

pub fn get_chaos_options() -> NodeOptions {
    let mut opts = get_mainnet_options();
    opts.network = "chaos".into();
    opts
}

pub fn get_debug_options() -> NodeOptions {
    let mut opts = get_mainnet_options();
    opts.network = "debug".into();
    opts
}

pub fn get_simulator_options() -> NodeOptions {
    NodeOptions {
        heartbeat_interval: Duration::from_millis(300),
        num_peers: 8,
        default_punish: 0,
        no_response_punish: 0,
        invalid_data_punish: 0,
        incorrect_power_punish: 0,
        max_punish: 0,
        outdated_heights_threshold: 5,
        state_unavailable_ban_time: 10,
        ip_request_limit_per_minute: 6000,
        network: "simulator".into(),
    }
}
