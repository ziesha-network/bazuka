use super::*;

mod simulation;

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;

#[tokio::test]
async fn test_timestamps_are_sync() {
    let enabled = Arc::new(RwLock::new(true));
    let (node_futs, route_futs, chans) = simulation::test_network(Arc::clone(&enabled), 3);
    let test_logic = async {
        sleep(Duration::from_millis(2000)).await;

        let mut timestamps = Vec::new();
        for chan in chans.values() {
            timestamps.push(chan.stats().await.unwrap().timestamp);
        }
        let first = timestamps.first().unwrap();
        assert!(timestamps.iter().all(|t| t == first));

        for chan in chans.values() {
            chan.shutdown().await.unwrap();
        }
    };
    tokio::join!(node_futs, route_futs, test_logic);
}
