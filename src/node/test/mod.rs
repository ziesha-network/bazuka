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

async fn catch_change<F: Fn() -> Fut, T, Fut>(f: F, timeout: usize) -> Result<T, NodeError>
where
    Fut: futures::Future<Output = Result<T, NodeError>>,
    T: std::fmt::Display + PartialEq,
{
    let prev_val = f().await?;
    for _ in 0..timeout {
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
                auto_gen_block: false,
                mpn_workers: vec![],
            },
            NodeOpts {
                config: conf.clone(),
                wallet: TxBuilder::new(&Vec::from("VALIDATOR2")),
                addr: 121,
                bootstrap: vec![120],
                timestamp_offset: 10,
                auto_gen_block: false,
                mpn_workers: vec![],
            },
            NodeOpts {
                config: conf.clone(),
                wallet: TxBuilder::new(&Vec::from("VALIDATOR3")),
                addr: 122,
                bootstrap: vec![121],
                timestamp_offset: 15,
                auto_gen_block: false,
                mpn_workers: vec![],
            },
        ],
    );
    let test_logic = async {
        assert!(
            catch_change(
                || async {
                    let mut peer_counts = Vec::new();
                    for chan in chans.iter() {
                        peer_counts.push(chan.peers().await?.peers.len());
                    }
                    Ok(peer_counts.into_iter().all(|c| c == 2))
                },
                MAX_WAIT_FOR_CHANGE
            )
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
                auto_gen_block: false,
                mpn_workers: vec![],
            },
            NodeOpts {
                config: conf.clone(),
                wallet: TxBuilder::new(&Vec::from("VALIDATOR2")),
                addr: 121,
                bootstrap: vec![120],
                timestamp_offset: 10,
                auto_gen_block: false,
                mpn_workers: vec![],
            },
            NodeOpts {
                config: conf.clone(),
                wallet: TxBuilder::new(&Vec::from("VALIDATOR3")),
                addr: 122,
                bootstrap: vec![121],
                timestamp_offset: 15,
                auto_gen_block: false,
                mpn_workers: vec![],
            },
        ],
    );
    let test_logic = async {
        assert!(
            catch_change(
                || async {
                    let mut timestamps = Vec::new();
                    for chan in chans.iter() {
                        timestamps.push(chan.stats().await?.timestamp);
                    }
                    let first = timestamps.first().unwrap();
                    Ok(timestamps.iter().all(|t| t == first))
                },
                MAX_WAIT_FOR_CHANGE
            )
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
                auto_gen_block: false,
                mpn_workers: vec![],
            },
            NodeOpts {
                config: conf.clone(),
                wallet: TxBuilder::new(&Vec::from("VALIDATOR2")),
                addr: 121,
                bootstrap: vec![120],
                timestamp_offset: 10,
                auto_gen_block: false,
                mpn_workers: vec![],
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
        assert!(
            catch_change(
                || async { Ok(chans[0].stats().await?.height == 50) },
                MAX_WAIT_FOR_CHANGE
            )
            .await?,
        );
        assert_eq!(chans[1].stats().await?.height, 50);

        // Now nodes should immediately sync with post_block
        chans[1].mine().await?;
        assert_eq!(chans[1].stats().await?.height, 51);
        assert!(
            catch_change(
                || async { Ok(chans[0].stats().await?.height == 51) },
                MAX_WAIT_FOR_CHANGE
            )
            .await?,
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
async fn test_auto_block_production() -> Result<(), NodeError> {
    init();

    let rules = Arc::new(RwLock::new(vec![]));

    let mut conf = blockchain::get_test_blockchain_config();
    conf.slot_duration = 2;
    conf.mpn_config.mpn_num_deposit_batches = 1;
    conf.mpn_config.mpn_num_withdraw_batches = 1;
    conf.mpn_config.mpn_num_update_batches = 1;
    let abc = TxBuilder::new(&Vec::from("ABC"));

    let val1 = TxBuilder::new(&Vec::from("VALIDATOR"));
    let val2 = TxBuilder::new(&Vec::from("VALIDATOR2"));
    let val3 = TxBuilder::new(&Vec::from("VALIDATOR3"));

    let (node_futs, route_futs, chans) = simulation::test_network(
        Arc::clone(&rules),
        vec![
            NodeOpts {
                config: conf.clone(),
                wallet: val1.clone(),
                addr: 120,
                bootstrap: vec![121, 122],
                timestamp_offset: 0,
                auto_gen_block: true,
                mpn_workers: vec![MpnWorker {
                    mpn_address: abc.get_mpn_address(),
                }],
            },
            NodeOpts {
                config: conf.clone(),
                wallet: val2.clone(),
                addr: 121,
                bootstrap: vec![120, 122],
                timestamp_offset: 0,
                auto_gen_block: true,
                mpn_workers: vec![MpnWorker {
                    mpn_address: abc.get_mpn_address(),
                }],
            },
            NodeOpts {
                config: conf.clone(),
                wallet: val3.clone(),
                addr: 122,
                bootstrap: vec![120, 121],
                timestamp_offset: 0,
                auto_gen_block: true,
                mpn_workers: vec![MpnWorker {
                    mpn_address: abc.get_mpn_address(),
                }],
            },
        ],
    );
    let test_logic = async {
        let mut height = 1;
        for _ in 0..10 {
            let next_height = catch_change(
                || async {
                    // Continuously post dummy proofs to all validators to ensure block production
                    for ch in chans.iter() {
                        ch.post_mpn_proof(
                            [
                                (0, zk::ZkProof::Dummy(true)),
                                (1, zk::ZkProof::Dummy(true)),
                                (2, zk::ZkProof::Dummy(true)),
                            ]
                            .into_iter()
                            .collect(),
                        )
                        .await?;
                    }
                    Ok(chans[0].stats().await?.height)
                },
                MAX_WAIT_FOR_CHANGE,
            )
            .await?;
            assert!(next_height > height);
            height = next_height;
        }

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

    let mpn_conf = blockchain::get_test_blockchain_config().mpn_config;
    let cid = mpn_conf.mpn_contract_id;
    let mut full_state = zk::ZkState {
        rollbacks: vec![],
        data: zk::ZkDataPairs(Default::default()),
    };
    let state_delta = zk::ZkDeltaPairs(
        [(zk::ZkDataLocator(vec![0, 0]), Some(zk::ZkScalar::from(234)))]
            .into_iter()
            .collect(),
    );
    full_state.apply_delta(&state_delta);
    updater.call_function(
        "".into(),
        cid,
        0,
        state_delta.clone(),
        mpn_conf
            .state_model()
            .compress::<ZkHasher>(&full_state.data)
            .unwrap(),
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
                auto_gen_block: false,
                mpn_workers: vec![],
            },
            NodeOpts {
                config: conf.clone(),
                wallet: TxBuilder::new(&Vec::from("VALIDATOR2")),
                addr: 121,
                bootstrap: vec![120],
                timestamp_offset: 10,
                auto_gen_block: false,
                mpn_workers: vec![],
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
            catch_change(
                || async { Ok(chans[1].stats().await?.height) },
                MAX_WAIT_FOR_CHANGE
            )
            .await?,
            2
        );

        assert_eq!(chans[0].outdated_heights().await?.outdated_heights.len(), 0);
        assert_eq!(chans[1].outdated_heights().await?.outdated_heights.len(), 1);

        // Now we open transmission of everything
        rules.write().await.clear();
        assert_eq!(chans[0].outdated_heights().await?.outdated_heights.len(), 0);
        assert_eq!(
            catch_change(
                || async { Ok(chans[1].outdated_heights().await?.outdated_heights.len()) },
                MAX_WAIT_FOR_CHANGE
            )
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
                auto_gen_block: false,
                mpn_workers: vec![],
            },
            NodeOpts {
                config: conf.clone(),
                wallet: TxBuilder::new(&Vec::from("VALIDATOR2")),
                addr: 121,
                bootstrap: vec![120],
                timestamp_offset: 10,
                auto_gen_block: false,
                mpn_workers: vec![],
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
            catch_change(
                || async { Ok(chans[1].stats().await?.height) },
                MAX_WAIT_FOR_CHANGE
            )
            .await?,
            2
        );
        assert_eq!(chans[0].outdated_heights().await?.outdated_heights.len(), 0);
        assert_eq!(chans[1].outdated_heights().await?.outdated_heights.len(), 1);

        assert!(chans[1].mine().await?.success == false);

        assert_eq!(
            catch_change(
                || async { Ok(chans[1].stats().await?.height) },
                MAX_WAIT_FOR_CHANGE
            )
            .await?,
            1
        );
        assert_eq!(chans[1].outdated_heights().await?.outdated_heights.len(), 0);

        // Header will be banned for some time and gets unbanned again:
        assert_eq!(
            catch_change(
                || async { Ok(chans[1].stats().await?.height) },
                MAX_WAIT_FOR_CHANGE
            )
            .await?,
            2
        );
        assert_eq!(chans[1].outdated_heights().await?.outdated_heights.len(), 1);

        // Banned again...
        assert_eq!(
            catch_change(
                || async { Ok(chans[1].stats().await?.height) },
                MAX_WAIT_FOR_CHANGE
            )
            .await?,
            1
        );
        assert_eq!(chans[1].outdated_heights().await?.outdated_heights.len(), 0);

        chans[1].mine().await?;
        chans[1].mine().await?;
        assert_eq!(chans[1].stats().await?.height, 3);
        assert_eq!(chans[1].outdated_heights().await?.outdated_heights.len(), 0);

        assert_eq!(
            catch_change(
                || async { Ok(chans[0].stats().await?.height) },
                MAX_WAIT_FOR_CHANGE
            )
            .await?,
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
