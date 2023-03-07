use crate::node::{HeartbeatIntervals, NodeOptions};
use std::time::Duration;

pub fn get_node_options() -> NodeOptions {
    NodeOptions {
        tx_max_time_alive: Some(600),
        heartbeat_intervals: HeartbeatIntervals {
            log_info: Duration::from_secs(5),
            refresh: Duration::from_secs(10),
            sync_peers: Duration::from_secs(60),
            discover_peers: Duration::from_secs(10),
            sync_clock: Duration::from_secs(10),
            sync_blocks: Duration::from_secs(10),
            sync_mempool: Duration::from_secs(30),
            sync_state: Duration::from_secs(10),
            generate_block: Duration::from_secs(3),
        },
        num_peers: 8,
        max_blocks_fetch: 16,
        default_punish: 60,
        no_response_punish: 600,
        invalid_data_punish: 3600,
        max_punish: 7200,
        incorrect_chain_punish: 3600,
        outdated_heights_threshold: 120,
        state_unavailable_ban_time: 30,
        candidate_remove_threshold: 3600,
        chain_mempool_max_fetch: 1000,
        mpn_mempool_max_fetch: 1000,
        max_block_time_difference: 120,
        automatic_block_generation: true,
    }
}

pub fn get_simulator_options() -> NodeOptions {
    NodeOptions {
        tx_max_time_alive: None,
        heartbeat_intervals: HeartbeatIntervals {
            log_info: Duration::from_secs(1),
            refresh: Duration::from_millis(300),
            sync_peers: Duration::from_millis(300),
            discover_peers: Duration::from_millis(300),
            sync_clock: Duration::from_millis(300),
            sync_blocks: Duration::from_millis(300),
            sync_mempool: Duration::from_millis(300),
            sync_state: Duration::from_millis(300),
            generate_block: Duration::from_millis(300),
        },
        num_peers: 8,
        max_blocks_fetch: 16,
        default_punish: 0,
        no_response_punish: 0,
        invalid_data_punish: 0,
        max_punish: 0,
        incorrect_chain_punish: 0,
        outdated_heights_threshold: 5,
        state_unavailable_ban_time: 10,
        candidate_remove_threshold: 600,
        chain_mempool_max_fetch: 1000,
        mpn_mempool_max_fetch: 1000,
        max_block_time_difference: 120,
        automatic_block_generation: false,
    }
}
