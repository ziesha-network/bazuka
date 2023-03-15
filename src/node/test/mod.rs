use super::*;

mod simulation;
use simulation::*;

use crate::config::blockchain;
use crate::core::{Money, TransactionAndDelta, ZkHasher};
use crate::zk;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;

fn init() {
    let _ = env_logger::builder().is_test(true).try_init();
}

const MAX_WAIT_FOR_CHANGE: usize = 20;

async fn catch_change<F: Fn() -> Fut, T, Fut>(f: F) -> Result<T, NodeError>
where
    Fut: futures::Future<Output = Result<T, NodeError>>,
    T: std::fmt::Display + PartialEq,
{
    let prev_val = f().await?;
    for _ in 0..MAX_WAIT_FOR_CHANGE {
        sleep(Duration::from_secs(1)).await;
        let new_val = f().await?;
        if new_val != prev_val {
            return Ok(new_val);
        }
    }
    Ok(prev_val)
}

#[tokio::test]
async fn test_peers_find_each_other() -> Result<(), NodeError> {
    init();

    let rules = Arc::new(RwLock::new(Vec::new()));
    let conf = blockchain::get_test_blockchain_config();

    let (node_futs, route_futs, chans) = simulation::test_network(
        Arc::clone(&rules),
        vec![
            NodeOpts {
                config: conf.clone(),
                wallet: TxBuilder::new(&Vec::from("VALIDATOR")),
                addr: 120,
                bootstrap: vec![],
                timestamp_offset: 5,
            },
            NodeOpts {
                config: conf.clone(),
                wallet: TxBuilder::new(&Vec::from("VALIDATOR2")),
                addr: 121,
                bootstrap: vec![120],
                timestamp_offset: 10,
            },
            NodeOpts {
                config: conf.clone(),
                wallet: TxBuilder::new(&Vec::from("VALIDATOR3")),
                addr: 122,
                bootstrap: vec![121],
                timestamp_offset: 15,
            },
        ],
    );
    let test_logic = async {
        assert!(
            catch_change(|| async {
                let mut peer_counts = Vec::new();
                for chan in chans.iter() {
                    peer_counts.push(chan.peers().await?.peers.len());
                }
                Ok(peer_counts.into_iter().all(|c| c == 2))
            })
            .await?
        );

        for chan in chans.iter() {
            chan.shutdown().await?;
        }
        Ok::<(), NodeError>(())
    };
    tokio::try_join!(node_futs, route_futs, test_logic)?;
    Ok(())
}

#[tokio::test]
async fn test_timestamps_are_sync() -> Result<(), NodeError> {
    init();

    let rules = Arc::new(RwLock::new(Vec::new()));
    let conf = blockchain::get_test_blockchain_config();

    let (node_futs, route_futs, chans) = simulation::test_network(
        Arc::clone(&rules),
        vec![
            NodeOpts {
                config: conf.clone(),
                wallet: TxBuilder::new(&Vec::from("VALIDATOR")),
                addr: 120,
                bootstrap: vec![],
                timestamp_offset: 5,
            },
            NodeOpts {
                config: conf.clone(),
                wallet: TxBuilder::new(&Vec::from("VALIDATOR2")),
                addr: 121,
                bootstrap: vec![120],
                timestamp_offset: 10,
            },
            NodeOpts {
                config: conf.clone(),
                wallet: TxBuilder::new(&Vec::from("VALIDATOR3")),
                addr: 122,
                bootstrap: vec![121],
                timestamp_offset: 15,
            },
        ],
    );
    let test_logic = async {
        assert!(
            catch_change(|| async {
                let mut timestamps = Vec::new();
                for chan in chans.iter() {
                    timestamps.push(chan.stats().await?.timestamp);
                }
                let first = timestamps.first().unwrap();
                Ok(timestamps.iter().all(|t| t == first))
            })
            .await?
        );

        for chan in chans.iter() {
            chan.shutdown().await?;
        }
        Ok::<(), NodeError>(())
    };
    tokio::try_join!(node_futs, route_futs, test_logic)?;
    Ok(())
}

#[tokio::test]
async fn test_blocks_get_synced() -> Result<(), NodeError> {
    init();

    // Allow sync of clocks but no block transfer
    let rules = Arc::new(RwLock::new(vec![]));

    let conf = blockchain::get_test_blockchain_config();

    let (node_futs, route_futs, chans) = simulation::test_network(
        Arc::clone(&rules),
        vec![
            NodeOpts {
                config: conf.clone(),
                wallet: TxBuilder::new(&Vec::from("VALIDATOR")),
                addr: 120,
                bootstrap: vec![],
                timestamp_offset: 5,
            },
            NodeOpts {
                config: conf.clone(),
                wallet: TxBuilder::new(&Vec::from("VALIDATOR2")),
                addr: 121,
                bootstrap: vec![120],
                timestamp_offset: 10,
            },
        ],
    );
    let test_logic = async {
        // Wait till clocks sync
        sleep(Duration::from_millis(1000)).await;

        *rules.write().await = vec![Rule::drop_all()];

        chans[0].mine().await?;
        assert_eq!(chans[0].stats().await?.height, 2);
        chans[0].mine().await?;
        assert_eq!(chans[0].stats().await?.height, 3);
        chans[0].mine().await?;
        assert_eq!(chans[0].stats().await?.height, 4);

        for i in 2..51 {
            chans[1].mine().await?;
            assert_eq!(chans[1].stats().await?.height, i);
        }

        // Still not synced...
        sleep(Duration::from_millis(2000)).await;
        assert_eq!(chans[0].stats().await?.height, 4);
        assert_eq!(chans[1].stats().await?.height, 50);

        // Now we open the connections...
        rules.write().await.clear();
        assert!(catch_change(|| async { Ok(chans[0].stats().await?.height == 50) }).await?,);
        assert_eq!(chans[1].stats().await?.height, 50);

        // Now nodes should immediately sync with post_block
        chans[1].mine().await?;
        assert_eq!(chans[1].stats().await?.height, 51);
        assert!(catch_change(|| async { Ok(chans[0].stats().await?.height == 51) }).await?,);

        for chan in chans.iter() {
            chan.shutdown().await?;
        }

        Ok::<(), NodeError>(())
    };
    tokio::try_join!(node_futs, route_futs, test_logic)?;
    Ok(())
}

fn sample_contract_call() -> TransactionAndDelta {
    let updater = TxBuilder::new(&Vec::from("ABC"));

    let cid = blockchain::get_test_blockchain_config()
        .mpn_config
        .mpn_contract_id;
    let state_model = zk::ZkStateModel::List {
        item_type: Box::new(zk::ZkStateModel::Scalar),
        log4_size: 5,
    };
    let mut full_state = zk::ZkState {
        rollbacks: vec![],
        data: zk::ZkDataPairs(
            [(zk::ZkDataLocator(vec![100]), zk::ZkScalar::from(200))]
                .into_iter()
                .collect(),
        ),
    };
    let state_delta = zk::ZkDeltaPairs(
        [(zk::ZkDataLocator(vec![123]), Some(zk::ZkScalar::from(234)))]
            .into_iter()
            .collect(),
    );
    full_state.apply_delta(&state_delta);
    updater.call_function(
        "".into(),
        cid,
        0,
        state_delta.clone(),
        state_model.compress::<ZkHasher>(&full_state.data).unwrap(),
        zk::ZkProof::Dummy(true),
        Money::ziesha(0),
        Money::ziesha(0),
        1,
    )
}

#[tokio::test]
async fn test_states_get_synced() -> Result<(), NodeError> {
    init();

    let rules = Arc::new(RwLock::new(vec![Rule::drop_all()]));
    let conf = blockchain::get_test_blockchain_config();

    let (node_futs, route_futs, chans) = simulation::test_network(
        Arc::clone(&rules),
        vec![
            NodeOpts {
                config: conf.clone(),
                wallet: TxBuilder::new(&Vec::from("VALIDATOR")),
                addr: 120,
                bootstrap: vec![],
                timestamp_offset: 5,
            },
            NodeOpts {
                config: conf.clone(),
                wallet: TxBuilder::new(&Vec::from("VALIDATOR2")),
                addr: 121,
                bootstrap: vec![120],
                timestamp_offset: 10,
            },
        ],
    );
    let test_logic = async {
        let tx_delta = sample_contract_call();

        chans[0].transact(tx_delta).await?;

        chans[0].mine().await?;
        assert_eq!(chans[0].stats().await?.height, 2);

        assert_eq!(chans[0].outdated_heights().await?.outdated_heights.len(), 0);

        // Still not synced...
        sleep(Duration::from_millis(1000)).await;
        assert_eq!(chans[0].stats().await?.height, 2);
        assert_eq!(chans[1].stats().await?.height, 1);

        // Now we open the connections but prevent transmission of states...
        *rules.write().await = vec![Rule::drop_url("state")];
        assert_eq!(chans[0].stats().await?.height, 2);
        assert_eq!(
            catch_change(|| async { Ok(chans[1].stats().await?.height) }).await?,
            2
        );

        assert_eq!(chans[0].outdated_heights().await?.outdated_heights.len(), 0);
        assert_eq!(chans[1].outdated_heights().await?.outdated_heights.len(), 1);

        // Now we open transmission of everything
        rules.write().await.clear();
        assert_eq!(chans[0].outdated_heights().await?.outdated_heights.len(), 0);
        assert_eq!(
            catch_change(|| async {
                Ok(chans[1].outdated_heights().await?.outdated_heights.len())
            })
            .await?,
            0
        );

        for chan in chans.iter() {
            chan.shutdown().await?;
        }

        Ok::<(), NodeError>(())
    };
    tokio::try_join!(node_futs, route_futs, test_logic)?;
    Ok(())
}

#[tokio::test]
async fn test_chain_rolls_back() -> Result<(), NodeError> {
    init();

    let rules = Arc::new(RwLock::new(vec![Rule::drop_all()]));
    let conf = blockchain::get_test_blockchain_config();

    let (node_futs, route_futs, chans) = simulation::test_network(
        Arc::clone(&rules),
        vec![
            NodeOpts {
                config: conf.clone(),
                wallet: TxBuilder::new(&Vec::from("VALIDATOR")),
                addr: 120,
                bootstrap: vec![],
                timestamp_offset: 5,
            },
            NodeOpts {
                config: conf.clone(),
                wallet: TxBuilder::new(&Vec::from("VALIDATOR2")),
                addr: 121,
                bootstrap: vec![120],
                timestamp_offset: 10,
            },
        ],
    );
    let test_logic = async {
        let tx_delta = sample_contract_call();

        chans[0].transact(tx_delta).await?;

        chans[0].mine().await?;
        assert_eq!(chans[0].stats().await?.height, 2);

        *rules.write().await = vec![Rule::drop_url("state")];
        assert_eq!(
            catch_change(|| async { Ok(chans[1].stats().await?.height) }).await?,
            2
        );
        assert_eq!(chans[0].outdated_heights().await?.outdated_heights.len(), 0);
        assert_eq!(chans[1].outdated_heights().await?.outdated_heights.len(), 1);

        assert!(chans[1].mine().await?.success == false);

        assert_eq!(
            catch_change(|| async { Ok(chans[1].stats().await?.height) }).await?,
            1
        );
        assert_eq!(chans[1].outdated_heights().await?.outdated_heights.len(), 0);

        // Header will be banned for some time and gets unbanned again:
        assert_eq!(
            catch_change(|| async { Ok(chans[1].stats().await?.height) }).await?,
            2
        );
        assert_eq!(chans[1].outdated_heights().await?.outdated_heights.len(), 1);

        // Banned again...
        assert_eq!(
            catch_change(|| async { Ok(chans[1].stats().await?.height) }).await?,
            1
        );
        assert_eq!(chans[1].outdated_heights().await?.outdated_heights.len(), 0);

        chans[1].mine().await?;
        chans[1].mine().await?;
        assert_eq!(chans[1].stats().await?.height, 3);
        assert_eq!(chans[1].outdated_heights().await?.outdated_heights.len(), 0);

        assert_eq!(
            catch_change(|| async { Ok(chans[0].stats().await?.height) }).await?,
            3
        );
        assert_eq!(chans[0].stats().await?.height, 3);
        assert_eq!(chans[0].outdated_heights().await?.outdated_heights.len(), 0);

        for chan in chans.iter() {
            chan.shutdown().await?;
        }

        Ok::<(), NodeError>(())
    };
    tokio::try_join!(node_futs, route_futs, test_logic)?;
    Ok(())
}
