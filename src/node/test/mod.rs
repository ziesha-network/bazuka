use super::*;

mod simulation;

use crate::config::genesis;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;

#[tokio::test]
async fn test_timestamps_are_sync() {
    let enabled = Arc::new(RwLock::new(true));
    let (node_futs, route_futs, chans) = simulation::test_network(
        Arc::clone(&enabled),
        genesis::get_genesis_block(),
        vec![None, None, None],
    );
    let test_logic = async {
        sleep(Duration::from_millis(2000)).await;

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
    let enabled = Arc::new(RwLock::new(false));

    let wallet0 = Some(Wallet::new(Vec::from("ABC")));
    let wallet1 = Some(Wallet::new(Vec::from("CBA")));

    let (node_futs, route_futs, chans) = simulation::test_network(
        Arc::clone(&enabled),
        genesis::get_test_genesis_block(),
        vec![wallet0, wallet1],
    );
    let test_logic = async {
        chans[0].set_miner(None).await.unwrap();
        chans[0].mine().await.unwrap();
        assert_eq!(chans[0].stats().await.unwrap().height, 2);
        chans[0].mine().await.unwrap();
        assert_eq!(chans[0].stats().await.unwrap().height, 3);
        chans[0].mine().await.unwrap();
        assert_eq!(chans[0].stats().await.unwrap().height, 4);

        chans[1].set_miner(None).await.unwrap();
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
