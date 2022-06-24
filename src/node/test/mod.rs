use super::*;

mod simulation;
use simulation::*;

use crate::blockchain::BlockchainError;
use crate::config::blockchain;
use crate::core::{ContractId, TransactionAndDelta, ZkHasher};
use crate::zk;
use std::str::FromStr;
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
    let conf = blockchain::get_blockchain_config();

    let (node_futs, route_futs, chans) = simulation::test_network(
        Arc::clone(&rules),
        vec![
            NodeOpts {
                config: conf.clone(),
                wallet: None,
                addr: 3030,
                bootstrap: vec![],
                timestamp_offset: 5,
            },
            NodeOpts {
                config: conf.clone(),
                wallet: None,
                addr: 3031,
                bootstrap: vec![3030],
                timestamp_offset: 10,
            },
            NodeOpts {
                config: conf.clone(),
                wallet: None,
                addr: 3032,
                bootstrap: vec![3031],
                timestamp_offset: 15,
            },
        ],
    );
    let test_logic = async {
        sleep(Duration::from_millis(1000)).await;

        for chan in chans.iter() {
            assert_eq!(chan.peers().await?.peers.len(), 2);
        }

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
    let conf = blockchain::get_blockchain_config();

    let (node_futs, route_futs, chans) = simulation::test_network(
        Arc::clone(&rules),
        vec![
            NodeOpts {
                config: conf.clone(),
                wallet: None,
                addr: 3030,
                bootstrap: vec![],
                timestamp_offset: 5,
            },
            NodeOpts {
                config: conf.clone(),
                wallet: None,
                addr: 3031,
                bootstrap: vec![3030],
                timestamp_offset: 10,
            },
            NodeOpts {
                config: conf.clone(),
                wallet: None,
                addr: 3032,
                bootstrap: vec![3031],
                timestamp_offset: 15,
            },
        ],
    );
    let test_logic = async {
        sleep(Duration::from_millis(1000)).await;

        let mut timestamps = Vec::new();
        for chan in chans.iter() {
            timestamps.push(chan.stats().await?.timestamp);
        }
        let first = timestamps.first().unwrap();
        assert!(timestamps.iter().all(|t| t == first));

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
                wallet: Some(Wallet::new(Vec::from("ABC"))),
                addr: 3030,
                bootstrap: vec![],
                timestamp_offset: 5,
            },
            NodeOpts {
                config: conf.clone(),
                wallet: Some(Wallet::new(Vec::from("CBA"))),
                addr: 3031,
                bootstrap: vec![3030],
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

        chans[1].mine().await?;
        assert_eq!(chans[1].stats().await?.height, 2);
        chans[1].mine().await?;
        assert_eq!(chans[1].stats().await?.height, 3);
        chans[1].mine().await?;
        assert_eq!(chans[1].stats().await?.height, 4);
        chans[1].mine().await?;
        assert_eq!(chans[1].stats().await?.height, 5);
        chans[1].mine().await?;
        assert_eq!(chans[1].stats().await?.height, 6);

        // Still not synced...
        sleep(Duration::from_millis(2000)).await;
        assert_eq!(chans[0].stats().await?.height, 4);
        assert_eq!(chans[1].stats().await?.height, 6);

        // Now we open the connections...
        rules.write().await.clear();
        assert_eq!(
            catch_change(|| async { Ok(chans[0].stats().await?.height) }).await?,
            6
        );
        assert_eq!(chans[1].stats().await?.height, 6);

        // Now nodes should immediately sync with post_block
        chans[1].mine().await?;
        assert_eq!(chans[0].stats().await?.height, 7);
        assert_eq!(chans[1].stats().await?.height, 7);

        for chan in chans.iter() {
            chan.shutdown().await?;
        }

        Ok::<(), NodeError>(())
    };
    tokio::try_join!(node_futs, route_futs, test_logic)?;
    Ok(())
}

fn sample_contract_call() -> TransactionAndDelta {
    let updater = Wallet::new(Vec::from("ABC"));

    let cid =
        ContractId::from_str("b83f6f4d4e548b4dadd02e46ee0f6458f1fd713f2dcb34afb019afd00db0f7e7")
            .unwrap();
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
        cid,
        0,
        state_delta.clone(),
        full_state.compress::<ZkHasher>(2, state_model),
        zk::ZkProof::Dummy(true),
        0,
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
                wallet: Some(Wallet::new(Vec::from("ABC"))),
                addr: 3030,
                bootstrap: vec![],
                timestamp_offset: 5,
            },
            NodeOpts {
                config: conf.clone(),
                wallet: Some(Wallet::new(Vec::from("CBA"))),
                addr: 3031,
                bootstrap: vec![3030],
                timestamp_offset: 10,
            },
        ],
    );
    let test_logic = async {
        let tx_delta = sample_contract_call();

        chans[0].transact(tx_delta).await?;

        chans[0].mine().await?;
        assert_eq!(chans[0].stats().await?.height, 2);

        assert_eq!(chans[0].outdated_states().await?.outdated_states.len(), 0);

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

        assert_eq!(chans[0].outdated_states().await?.outdated_states.len(), 0);
        assert_eq!(chans[1].outdated_states().await?.outdated_states.len(), 1);

        // Now we open transmission of everything
        rules.write().await.clear();
        assert_eq!(chans[0].outdated_states().await?.outdated_states.len(), 0);
        assert_eq!(
            catch_change(|| async { Ok(chans[1].outdated_states().await?.outdated_states.len()) })
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
                wallet: Some(Wallet::new(Vec::from("ABC"))),
                addr: 3030,
                bootstrap: vec![],
                timestamp_offset: 5,
            },
            NodeOpts {
                config: conf.clone(),
                wallet: Some(Wallet::new(Vec::from("CBA"))),
                addr: 3031,
                bootstrap: vec![3030],
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
        assert_eq!(chans[0].outdated_states().await?.outdated_states.len(), 0);
        assert_eq!(chans[1].outdated_states().await?.outdated_states.len(), 1);

        assert!(matches!(
            chans[1].mine().await,
            Err(NodeError::BlockchainError(BlockchainError::StatesOutdated))
        ));

        assert_eq!(
            catch_change(|| async { Ok(chans[1].stats().await?.height) }).await?,
            1
        );
        assert_eq!(chans[1].outdated_states().await?.outdated_states.len(), 0);

        // Header will be banned for some time and gets unbanned again:
        assert_eq!(
            catch_change(|| async { Ok(chans[1].stats().await?.height) }).await?,
            2
        );
        assert_eq!(chans[1].outdated_states().await?.outdated_states.len(), 1);

        // Banned again...
        assert_eq!(
            catch_change(|| async { Ok(chans[1].stats().await?.height) }).await?,
            1
        );
        assert_eq!(chans[1].outdated_states().await?.outdated_states.len(), 0);

        chans[1].mine().await?;
        chans[1].mine().await?;
        assert_eq!(chans[1].stats().await?.height, 3);
        assert_eq!(chans[1].outdated_states().await?.outdated_states.len(), 0);

        assert_eq!(
            catch_change(|| async { Ok(chans[0].stats().await?.height) }).await?,
            3
        );
        assert_eq!(chans[0].stats().await?.height, 3);
        assert_eq!(chans[0].outdated_states().await?.outdated_states.len(), 0);

        for chan in chans.iter() {
            chan.shutdown().await?;
        }

        Ok::<(), NodeError>(())
    };
    tokio::try_join!(node_futs, route_futs, test_logic)?;
    Ok(())
}
