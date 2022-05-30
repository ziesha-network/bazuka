use super::*;

mod simulation;
use simulation::NodeOpts;

use crate::config::genesis;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;

fn init() {
    let _ = env_logger::builder().is_test(true).try_init();
}

#[tokio::test]
async fn test_peers_find_each_other() {
    init();

    let enabled = Arc::new(RwLock::new(true));
    let genesis = genesis::get_genesis_block();

    let (node_futs, route_futs, chans) = simulation::test_network(
        Arc::clone(&enabled),
        vec![
            NodeOpts {
                genesis: genesis.clone(),
                wallet: None,
                addr: 3030,
                bootstrap: vec![],
                timestamp_offset: 5,
            },
            NodeOpts {
                genesis: genesis.clone(),
                wallet: None,
                addr: 3031,
                bootstrap: vec![3030],
                timestamp_offset: 10,
            },
            NodeOpts {
                genesis: genesis.clone(),
                wallet: None,
                addr: 3032,
                bootstrap: vec![3031],
                timestamp_offset: 15,
            },
        ],
    );
    let test_logic = async {
        sleep(Duration::from_millis(5000)).await;

        for chan in chans.iter() {
            assert_eq!(chan.peers().await.unwrap().peers.len(), 2);
        }

        for chan in chans.iter() {
            chan.shutdown().await.unwrap();
        }
    };
    tokio::join!(node_futs, route_futs, test_logic);
}

#[tokio::test]
async fn test_timestamps_are_sync() {
    init();

    let enabled = Arc::new(RwLock::new(true));
    let genesis = genesis::get_genesis_block();

    let (node_futs, route_futs, chans) = simulation::test_network(
        Arc::clone(&enabled),
        vec![
            NodeOpts {
                genesis: genesis.clone(),
                wallet: None,
                addr: 3030,
                bootstrap: vec![],
                timestamp_offset: 5,
            },
            NodeOpts {
                genesis: genesis.clone(),
                wallet: None,
                addr: 3031,
                bootstrap: vec![3030],
                timestamp_offset: 10,
            },
            NodeOpts {
                genesis: genesis.clone(),
                wallet: None,
                addr: 3032,
                bootstrap: vec![3031],
                timestamp_offset: 15,
            },
        ],
    );
    let test_logic = async {
        sleep(Duration::from_millis(5000)).await;

        let mut timestamps = Vec::new();
        for chan in chans.iter() {
            timestamps.push(chan.stats().await.unwrap().timestamp);
        }
        let first = timestamps.first().unwrap();
        assert!(timestamps.iter().all(|t| t == first));

        for chan in chans.iter() {
            chan.shutdown().await.unwrap();
        }
    };
    tokio::join!(node_futs, route_futs, test_logic);
}

#[tokio::test]
async fn test_blocks_get_synced() {
    init();

    let enabled = Arc::new(RwLock::new(false));
    let genesis = genesis::get_test_genesis_block();

    let (node_futs, route_futs, chans) = simulation::test_network(
        Arc::clone(&enabled),
        vec![
            NodeOpts {
                genesis: genesis.clone(),
                wallet: Some(Wallet::new(Vec::from("ABC"))),
                addr: 3030,
                bootstrap: vec![],
                timestamp_offset: 5,
            },
            NodeOpts {
                genesis: genesis.clone(),
                wallet: Some(Wallet::new(Vec::from("CBA"))),
                addr: 3031,
                bootstrap: vec![3030],
                timestamp_offset: 10,
            },
        ],
    );
    let test_logic = async {
        chans[0].mine().await.unwrap();
        assert_eq!(chans[0].stats().await.unwrap().height, 2);
        chans[0].mine().await.unwrap();
        assert_eq!(chans[0].stats().await.unwrap().height, 3);
        chans[0].mine().await.unwrap();
        assert_eq!(chans[0].stats().await.unwrap().height, 4);

        chans[1].mine().await.unwrap();
        assert_eq!(chans[1].stats().await.unwrap().height, 2);
        chans[1].mine().await.unwrap();
        assert_eq!(chans[1].stats().await.unwrap().height, 3);
        chans[1].mine().await.unwrap();
        assert_eq!(chans[1].stats().await.unwrap().height, 4);
        chans[1].mine().await.unwrap();
        assert_eq!(chans[1].stats().await.unwrap().height, 5);
        chans[1].mine().await.unwrap();
        assert_eq!(chans[1].stats().await.unwrap().height, 6);

        // Still not synced...
        sleep(Duration::from_millis(2000)).await;
        assert_eq!(chans[0].stats().await.unwrap().height, 4);
        assert_eq!(chans[1].stats().await.unwrap().height, 6);

        // Now we open the connections...
        *enabled.write().await = true;
        sleep(Duration::from_millis(10000)).await;
        assert_eq!(chans[0].stats().await.unwrap().height, 6);
        assert_eq!(chans[1].stats().await.unwrap().height, 6);

        for chan in chans.iter() {
            chan.shutdown().await.unwrap();
        }
    };
    tokio::join!(node_futs, route_futs, test_logic);
}
