use crate::node::NodeOptions;
use std::time::Duration;

pub fn get_node_options() -> NodeOptions {
    NodeOptions {
        tx_max_time_alive: Some(600),
        heartbeat_interval: Duration::from_secs(10),
        num_peers: 8,
        max_blocks_fetch: 16,
        default_punish: 60,
        no_response_punish: 600,
        invalid_data_punish: 3600,
        incorrect_power_punish: 3600,
        max_punish: 7200,
        outdated_heights_threshold: 60,
        state_unavailable_ban_time: 30,
        candidate_remove_threshold: 600,
    }
}

pub fn get_simulator_options() -> NodeOptions {
    NodeOptions {
        tx_max_time_alive: None,
        heartbeat_interval: Duration::from_millis(300),
        num_peers: 8,
        max_blocks_fetch: 16,
        default_punish: 0,
        no_response_punish: 0,
        invalid_data_punish: 0,
        incorrect_power_punish: 0,
        max_punish: 0,
        outdated_heights_threshold: 5,
        state_unavailable_ban_time: 10,
        candidate_remove_threshold: 600,
    }
}
