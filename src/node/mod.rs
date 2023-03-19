#[cfg(test)]
mod test;

mod api;
mod context;
mod firewall;
mod heartbeat;
mod http;
mod peer_manager;
pub mod seeds;
use crate::blockchain::{BlockAndPatch, Blockchain, Mempool};
use crate::client::{
    messages::*, Limit, NodeError, NodeRequest, OutgoingSender, Peer, PeerAddress, Timestamp,
    NETWORK_HEADER, SIGNATURE_HEADER,
};
use crate::common::*;
use crate::crypto::ed25519;
use crate::crypto::SignatureScheme;
use crate::db::KvStore;
use crate::mpn::MpnWorker;
use crate::utils::local_timestamp;
use crate::wallet::TxBuilder;
use context::NodeContext;
pub use firewall::Firewall;
use hyper::body::HttpBody;
use hyper::{Body, Method, Request, Response, StatusCode};
use peer_manager::PeerManager;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tokio::try_join;

#[derive(Debug, Clone)]
pub struct HeartbeatIntervals {
    pub log_info: Duration,
    pub refresh: Duration,
    pub sync_peers: Duration,
    pub discover_peers: Duration,
    pub sync_clock: Duration,
    pub sync_blocks: Duration,
    pub sync_mempool: Duration,
    pub sync_state: Duration,
    pub generate_block: Duration,
}

#[derive(Debug, Clone)]
pub struct NodeOptions {
    pub tx_max_time_alive: Option<u32>,
    pub heartbeat_intervals: HeartbeatIntervals,
    pub num_peers: usize,
    pub max_blocks_fetch: u64,
    pub outdated_heights_threshold: u32,
    pub default_punish: u32,
    pub no_response_punish: u32,
    pub invalid_data_punish: u32,
    pub incorrect_chain_punish: u32,
    pub max_punish: u32,
    pub state_unavailable_ban_time: u32,
    pub candidate_remove_threshold: u32,
    pub chain_mempool_max_fetch: usize,
    pub mpn_mempool_max_fetch: usize,
    pub max_block_time_difference: u32,
    pub automatic_block_generation: bool,
}

fn fetch_signature(
    req: &Request<Body>,
) -> Result<Option<(ed25519::PublicKey, ed25519::Signature)>, NodeError> {
    if let Some(v) = req.headers().get(SIGNATURE_HEADER) {
        let s = v.to_str().map_err(|_| NodeError::InvalidSignatureHeader)?;
        let mut s = s.split('-');
        let (pub_hex, sig_hex) = s
            .next()
            .zip(s.next())
            .ok_or(NodeError::InvalidSignatureHeader)?;
        let pub_key = hex::decode(pub_hex)
            .map(|bytes| bincode::deserialize::<ed25519::PublicKey>(&bytes))
            .map_err(|_| NodeError::InvalidSignatureHeader)?
            .map_err(|_| NodeError::InvalidSignatureHeader)?;
        let sig = hex::decode(sig_hex)
            .map(|bytes| bincode::deserialize::<ed25519::Signature>(&bytes))
            .map_err(|_| NodeError::InvalidSignatureHeader)?
            .map_err(|_| NodeError::InvalidSignatureHeader)?;
        return Ok(Some((pub_key, sig)));
    }
    Ok(None)
}

async fn promote_block<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
    block_and_patch: BlockAndPatch,
) {
    let context = context.read().await;
    let net = context.outgoing.clone();
    let peer_addresses = context.peer_manager.get_peers();
    tokio::task::spawn(async move {
        http::group_request(&peer_addresses, |peer| {
            net.bincode_post::<PostBlockRequest, PostBlockResponse>(
                format!("http://{}/bincode/blocks", peer.address),
                PostBlockRequest {
                    block: block_and_patch.block.clone(),
                    patch: block_and_patch.patch.clone(),
                },
                Limit::default().size(KB).time(3 * SECOND),
            )
        })
        .await;
    });
}

async fn promote_validator_claim<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
    validator_claim: ValidatorClaim,
) {
    let context = context.read().await;
    let net = context.outgoing.clone();
    let peer_addresses = context.peer_manager.get_peers();
    tokio::task::spawn(async move {
        http::group_request(&peer_addresses, |peer| {
            net.bincode_post::<PostValidatorClaimRequest, PostValidatorClaimResponse>(
                format!("http://{}/claim", peer.address),
                PostValidatorClaimRequest {
                    validator_claim: validator_claim.clone(),
                },
                Limit::default().size(KB).time(1 * SECOND),
            )
        })
        .await;
    });
}

async fn node_service<K: KvStore, B: Blockchain<K>>(
    client: Option<SocketAddr>,
    context: Arc<RwLock<NodeContext<K, B>>>,
    req: Request<Body>,
) -> Result<Response<Body>, NodeError> {
    let is_local = client.map(|c| c.ip().is_loopback()).unwrap_or(true);
    match async {
        let mut response = Response::builder()
            .header("Access-Control-Allow-Origin", "*")
            .body(Body::default())?;

        if let Some(client) = client {
            let mut ctx = context.write().await;
            let now = ctx.local_timestamp();
            if ctx.peer_manager.is_ip_punished(now, client.ip()) {
                log::warn!("{} -> PeerManager dropped request!", client);
                *response.status_mut() = StatusCode::FORBIDDEN;
                return Ok(response);
            }
            if let Some(firewall) = &mut ctx.firewall {
                if !firewall.incoming_permitted(client) {
                    log::warn!("{} -> Firewall dropped request!", client);
                    *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
                    return Ok(response);
                }
            }
        }

        let method = req.method().clone();

        if method == Method::OPTIONS {
            return Ok(Response::builder()
                .status(StatusCode::OK)
                .header("Access-Control-Allow-Origin", "*")
                .header("Access-Control-Allow-Headers", "*")
                .header("Access-Control-Allow-Methods", "POST, GET, OPTIONS")
                .body(Body::default())?);
        }

        let path = req.uri().path().to_string();
        let qs = req.uri().query().unwrap_or("").to_string();

        log::info!(
            "{} -> {} {}",
            client
                .map(|c| c.to_string())
                .unwrap_or_else(|| "UNKNOWN".into()),
            method,
            req.uri()
        );

        let creds = fetch_signature(&req)?;
        let network: String = if let Some(v) = req.headers().get(NETWORK_HEADER) {
            v.to_str().ok().map(|n| n.to_lowercase())
        } else {
            None
        }
        .unwrap_or_else(|| "mainnet".into());

        let body = req.into_body();

        if !is_local && network != context.read().await.network {
            return Err(NodeError::WrongNetwork);
        }

        if let Some(req_sz) = body.size_hint().upper() {
            if let Some(client) = client {
                let mut ctx = context.write().await;
                if let Some(firewall) = &mut ctx.firewall {
                    firewall.add_traffic(client.ip(), req_sz);
                }
            }
        } else {
            *response.status_mut() = StatusCode::PAYLOAD_TOO_LARGE;
            return Ok(response);
        }

        let body_bytes = hyper::body::to_bytes(body).await?;

        let needs_signature = false;

        // TODO: This doesn't prevent replay attacks
        let is_signed = creds
            .map(|(pub_key, sig)| {
                ed25519::Ed25519::<crate::core::Hasher>::verify(&pub_key, &body_bytes, &sig)
            })
            .unwrap_or(false);
        if needs_signature && !is_signed {
            return Err(NodeError::SignatureRequired);
        }

        match (method, &path[..]) {
            #[cfg(test)]
            (Method::POST, "/generate_block") => {
                *response.body_mut() = Body::from(bincode::serialize(
                    &api::generate_block(Arc::clone(&context), bincode::deserialize(&body_bytes)?)
                        .await?,
                )?);
            }
            (Method::GET, "/stats") => {
                *response.body_mut() = Body::from(serde_json::to_vec(
                    &api::get_stats(Arc::clone(&context), serde_qs::from_str(&qs)?).await?,
                )?);
            }
            (Method::GET, "/debug") => {
                *response.body_mut() = Body::from(serde_json::to_vec(
                    &api::get_debug_data(Arc::clone(&context), serde_qs::from_str(&qs)?).await?,
                )?);
            }
            (Method::GET, "/account") => {
                *response.body_mut() = Body::from(serde_json::to_vec(
                    &api::get_account(Arc::clone(&context), serde_qs::from_str(&qs)?).await?,
                )?);
            }
            (Method::GET, "/delegations") => {
                *response.body_mut() = Body::from(serde_json::to_vec(
                    &api::get_delegations(Arc::clone(&context), serde_qs::from_str(&qs)?).await?,
                )?);
            }
            (Method::GET, "/balance") => {
                *response.body_mut() = Body::from(serde_json::to_vec(
                    &api::get_balance(Arc::clone(&context), serde_qs::from_str(&qs)?).await?,
                )?);
            }
            (Method::GET, "/mpn/account") => {
                *response.body_mut() = Body::from(serde_json::to_vec(
                    &api::get_mpn_account(Arc::clone(&context), serde_qs::from_str(&qs)?).await?,
                )?);
            }
            (Method::GET, "/peers") => {
                *response.body_mut() = Body::from(serde_json::to_vec(
                    &api::get_peers(client, Arc::clone(&context), serde_qs::from_str(&qs)?).await?,
                )?);
            }
            (Method::GET, "/explorer/stakers") => {
                *response.body_mut() = Body::from(serde_json::to_vec(
                    &api::get_explorer_stakers(Arc::clone(&context), serde_qs::from_str(&qs)?)
                        .await?,
                )?);
            }
            (Method::GET, "/token") => {
                *response.body_mut() = Body::from(serde_json::to_vec(
                    &api::get_token(Arc::clone(&context), serde_qs::from_str(&qs)?).await?,
                )?);
            }
            (Method::POST, "/bincode/peers") => {
                *response.body_mut() = Body::from(bincode::serialize(
                    &api::post_peer(
                        client,
                        Arc::clone(&context),
                        bincode::deserialize(&body_bytes)?,
                    )
                    .await?,
                )?);
            }
            (Method::POST, "/shutdown") => {
                if is_local {
                    *response.body_mut() = Body::from(serde_json::to_vec(
                        &api::shutdown(Arc::clone(&context), serde_json::from_slice(&body_bytes)?)
                            .await?,
                    )?);
                } else {
                    *response.status_mut() = StatusCode::FORBIDDEN;
                }
            }
            (Method::POST, "/bincode/transact") => {
                *response.body_mut() = Body::from(bincode::serialize(
                    &api::transact(
                        client,
                        Arc::clone(&context),
                        bincode::deserialize(&body_bytes)?,
                    )
                    .await?,
                )?);
            }
            (Method::POST, "/bincode/transact/zero") => {
                *response.body_mut() = Body::from(bincode::serialize(
                    &api::post_mpn_transaction(
                        client,
                        Arc::clone(&context),
                        bincode::deserialize(&body_bytes)?,
                    )
                    .await?,
                )?);
            }
            (Method::POST, "/transact/zero") => {
                *response.body_mut() = Body::from(bincode::serialize(
                    &api::post_mpn_transaction(
                        client,
                        Arc::clone(&context),
                        serde_json::from_slice::<
                            crate::client::messages::PostJsonMpnTransactionRequest,
                        >(&body_bytes)?
                        .try_into()?,
                    )
                    .await?,
                )?);
            }
            (Method::POST, "/bincode/transact/deposit") => {
                *response.body_mut() = Body::from(bincode::serialize(
                    &api::post_mpn_deposit(
                        client,
                        Arc::clone(&context),
                        bincode::deserialize(&body_bytes)?,
                    )
                    .await?,
                )?);
            }
            (Method::POST, "/bincode/transact/withdraw") => {
                *response.body_mut() = Body::from(bincode::serialize(
                    &api::post_mpn_withdraw(
                        client,
                        Arc::clone(&context),
                        bincode::deserialize(&body_bytes)?,
                    )
                    .await?,
                )?);
            }
            (Method::GET, "/explorer/blocks") => {
                *response.body_mut() = Body::from(serde_json::to_vec(
                    &api::get_explorer_blocks(Arc::clone(&context), serde_qs::from_str(&qs)?)
                        .await?,
                )?);
            }
            (Method::GET, "/explorer/mpn/accounts") => {
                *response.body_mut() = Body::from(serde_json::to_vec(
                    &api::get_explorer_mpn_accounts(Arc::clone(&context), serde_qs::from_str(&qs)?)
                        .await?,
                )?);
            }
            (Method::GET, "/explorer/mempool") => {
                *response.body_mut() = Body::from(serde_json::to_vec(
                    &api::get_explorer_mempool(Arc::clone(&context), serde_qs::from_str(&qs)?)
                        .await?,
                )?);
            }
            (Method::GET, "/bincode/headers") => {
                *response.body_mut() = Body::from(bincode::serialize(
                    &api::get_headers(Arc::clone(&context), bincode::deserialize(&body_bytes)?)
                        .await?,
                )?);
            }
            (Method::GET, "/bincode/blocks") => {
                *response.body_mut() = Body::from(bincode::serialize(
                    &api::get_blocks(Arc::clone(&context), bincode::deserialize(&body_bytes)?)
                        .await?,
                )?);
            }
            (Method::POST, "/bincode/blocks") => {
                *response.body_mut() = Body::from(bincode::serialize(
                    &api::post_block(Arc::clone(&context), bincode::deserialize(&body_bytes)?)
                        .await?,
                )?);
            }
            (Method::GET, "/bincode/states") => {
                *response.body_mut() = Body::from(bincode::serialize(
                    &api::get_states(Arc::clone(&context), bincode::deserialize(&body_bytes)?)
                        .await?,
                )?);
            }
            (Method::GET, "/bincode/states/outdated") => {
                *response.body_mut() = Body::from(bincode::serialize(
                    &api::get_outdated_heights(
                        Arc::clone(&context),
                        bincode::deserialize(&body_bytes)?,
                    )
                    .await?,
                )?);
            }
            (Method::GET, "/mempool") => {
                let req: GetJsonMempoolRequest = serde_qs::from_str(&qs)?;
                let mpn_address = req.mpn_address.parse()?;
                *response.body_mut() =
                    Body::from(serde_json::to_vec(&Into::<GetJsonMempoolResponse>::into(
                        api::get_mempool(
                            Arc::clone(&context),
                            GetMempoolRequest {},
                            Some(mpn_address),
                        )
                        .await?,
                    ))?);
            }
            (Method::GET, "/bincode/mempool") => {
                *response.body_mut() = Body::from(bincode::serialize(
                    &api::get_mempool(
                        Arc::clone(&context),
                        bincode::deserialize(&body_bytes)?,
                        None,
                    )
                    .await?,
                )?);
            }
            (Method::POST, "/claim") => {
                *response.body_mut() = Body::from(bincode::serialize(
                    &api::post_validator_claim(
                        Arc::clone(&context),
                        bincode::deserialize(&body_bytes)?,
                    )
                    .await?,
                )?);
            }
            (Method::GET, "/bincode/mpn/work") => {
                *response.body_mut() = Body::from(bincode::serialize(
                    &api::get_mpn_work(Arc::clone(&context), bincode::deserialize(&body_bytes)?)
                        .await?,
                )?);
            }
            (Method::POST, "/bincode/mpn/solution") => {
                *response.body_mut() = Body::from(bincode::serialize(
                    &api::post_mpn_solution(
                        Arc::clone(&context),
                        bincode::deserialize(&body_bytes)?,
                    )
                    .await?,
                )?);
            }
            (Method::POST, "/bincode/mpn/worker") => {
                *response.body_mut() = Body::from(bincode::serialize(
                    &api::post_mpn_worker(Arc::clone(&context), bincode::deserialize(&body_bytes)?)
                        .await?,
                )?);
            }
            _ => {
                *response.status_mut() = StatusCode::NOT_FOUND;
            }
        }

        if let Some(resp_sz) = response.body().size_hint().upper() {
            if let Some(client) = client {
                let mut ctx = context.write().await;
                if let Some(firewall) = &mut ctx.firewall {
                    firewall.add_traffic(client.ip(), resp_sz);
                }
            }
        }

        Ok::<Response<Body>, NodeError>(response)
    }
    .await
    {
        Ok(resp) => Ok(resp),
        Err(e) => {
            if let Some(client) = client {
                if !is_local {
                    let mut ctx = context.write().await;
                    let default_punish = ctx.opts.default_punish;
                    let now = ctx.local_timestamp();
                    ctx.peer_manager
                        .punish_ip_for(now, client.ip(), default_punish);
                }
            }
            log::warn!(
                "{} -> Error: {}",
                client
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "UNKNOWN".into()),
                e
            );
            Err(e)
        }
    }
}

use tokio::sync::mpsc;

pub async fn node_create<K: KvStore, B: Blockchain<K>>(
    opts: NodeOptions,
    network: &str,
    address: Option<PeerAddress>,
    bootstrap: Vec<PeerAddress>,
    blockchain: B,
    timestamp_offset: i32,
    validator_wallet: TxBuilder,
    user_wallet: TxBuilder,
    social_profiles: SocialProfiles,
    mut incoming: mpsc::UnboundedReceiver<NodeRequest>,
    outgoing: mpsc::UnboundedSender<NodeRequest>,
    firewall: Option<Firewall>,
    mpn_workers: Vec<MpnWorker>,
) -> Result<(), NodeError> {
    let context = Arc::new(RwLock::new(NodeContext {
        _phantom: std::marker::PhantomData,
        firewall,
        opts: opts.clone(),
        network: network.into(),
        social_profiles,
        address,
        shutdown: false,
        outgoing: Arc::new(OutgoingSender {
            network: network.into(),
            chan: outgoing,
            priv_key: validator_wallet.get_priv_key(),
        }),
        mpn_workers: mpn_workers
            .into_iter()
            .map(|w| (w.mpn_address.clone(), w))
            .collect(),
        mpn_work_pool: None,
        mempool: Mempool::new(blockchain.config().mpn_config.log4_tree_size),
        blockchain,
        validator_wallet,
        user_wallet,
        peer_manager: PeerManager::new(
            address,
            bootstrap,
            local_timestamp(),
            opts.candidate_remove_threshold,
        ),
        timestamp_offset,
        banned_headers: HashMap::new(),
        outdated_since: None,
        validator_claim: None,
    }));

    let server_future = async {
        loop {
            if context.read().await.shutdown {
                break;
            }
            if let Some(msg) = incoming.recv().await {
                if let Err(e) = msg
                    .resp
                    .send(node_service(msg.socket_addr, Arc::clone(&context), msg.body).await)
                    .await
                {
                    log::error!("Request sender not receiving its answer: {}", e);
                }
            } else {
                break;
            }
        }
        Ok(())
    };

    let heartbeat_future = heartbeat::heartbeater(Arc::clone(&context));

    try_join!(server_future, heartbeat_future)?;

    log::info!("Node stopped!");

    Ok(())
}
