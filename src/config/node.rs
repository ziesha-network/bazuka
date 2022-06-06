use crate::node::NodeOptions;
use std::time::Duration;

pub fn get_node_options() -> NodeOptions {
    NodeOptions {
        heartbeat_interval: Duration::from_secs(1),
        num_peers: 8,
        no_reponse_punish: 5,
        invalid_data_punish: 10,
        incorrect_power_punish: 12,
        max_punish: 15,
    }
}

pub fn get_test_node_options() -> NodeOptions {
    NodeOptions {
        heartbeat_interval: Duration::from_secs(1),
        num_peers: 8,
        no_reponse_punish: 1,
        invalid_data_punish: 1,
        incorrect_power_punish: 1,
        max_punish: 1,
    }
}
