#[cfg(test)]
mod test;

mod api;
mod context;
mod firewall;
mod heartbeat;
mod http;
pub mod seeds;
pub mod upnp;
use crate::blockchain::Blockchain;
use crate::client::{
    messages::SocialProfiles, Limit, NodeError, NodeRequest, OutgoingSender, Peer, PeerAddress,
    PeerInfo, Timestamp, NETWORK_HEADER, SIGNATURE_HEADER,
};
use crate::crypto::ed25519;
use crate::crypto::SignatureScheme;
use crate::utils::local_timestamp;
use crate::wallet::Wallet;
use context::NodeContext;
use firewall::Firewall;
use hyper::body::HttpBody;
use hyper::{Body, Method, Request, Response, StatusCode};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tokio::try_join;

#[derive(Debug, Clone)]
pub struct NodeOptions {
    pub tx_max_time_alive: Option<u32>,
    pub heartbeat_interval: Duration,
    pub num_peers: usize,
    pub max_blocks_fetch: u64,
    pub outdated_heights_threshold: u32,
    pub default_punish: u32,
    pub no_response_punish: u32,
    pub invalid_data_punish: u32,
    pub incorrect_power_punish: u32,
    pub max_punish: u32,
    pub state_unavailable_ban_time: u32,
    pub ip_request_limit_per_minute: usize,
    pub traffic_limit_per_15m: u64,
    pub unresponsive_count_limit_per_15m: usize,
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

async fn node_service<B: Blockchain>(
    client: Option<SocketAddr>,
    context: Arc<RwLock<NodeContext<B>>>,
    req: Request<Body>,
) -> Result<Response<Body>, NodeError> {
    match async {
        let is_local = client.map(|c| c.ip().is_loopback()).unwrap_or(true);
        let mut response = Response::new(Body::empty());

        let is_loopback = client.map(|c| c.ip().is_loopback()).unwrap_or(false);

        if let Some(client) = client {
            let mut ctx = context.write().await;
            if !ctx.firewall.incoming_permitted(client) {
                log::warn!("{} -> Firewall dropped request!", client);
                *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
                return Ok(response);
            }
        }

        let method = req.method().clone();
        let path = req.uri().path().to_string();
        let qs = req.uri().query().unwrap_or("").to_string();

        log::info!(
            "{} -> {} {}",
            client.map(|c| c.to_string()).unwrap_or("UNKNOWN".into()),
            method,
            req.uri()
        );

        let creds = fetch_signature(&req)?;
        let network: String = if let Some(v) = req.headers().get(NETWORK_HEADER) {
            v.to_str().ok().map(|n| n.to_lowercase())
        } else {
            None
        }
        .unwrap_or("mainnet".into());

        let body = req.into_body();

        if !is_loopback && network != context.read().await.network {
            return Err(NodeError::WrongNetwork);
        }

        if let Some(req_sz) = body.size_hint().upper() {
            if let Some(client) = client {
                context
                    .write()
                    .await
                    .firewall
                    .add_traffic(client.ip(), req_sz);
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
            // Miner will call this to fetch new PoW work.
            (Method::GET, "/miner/puzzle") => {
                *response.body_mut() = Body::from(serde_json::to_vec(
                    &api::get_miner_puzzle(Arc::clone(&context), serde_qs::from_str(&qs)?).await?,
                )?);
            }

            // Miner will call this when he has solved the PoW puzzle.
            (Method::POST, "/miner/solution") => {
                *response.body_mut() = Body::from(serde_json::to_vec(
                    &api::post_miner_solution(
                        Arc::clone(&context),
                        serde_json::from_slice(&body_bytes)?,
                    )
                    .await?,
                )?);
            }

            (Method::GET, "/stats") => {
                *response.body_mut() = Body::from(serde_json::to_vec(
                    &api::get_stats(Arc::clone(&context), serde_qs::from_str(&qs)?).await?,
                )?);
            }
            (Method::GET, "/account") => {
                *response.body_mut() = Body::from(serde_json::to_vec(
                    &api::get_account(Arc::clone(&context), serde_qs::from_str(&qs)?).await?,
                )?);
            }
            (Method::GET, "/mpn/account") => {
                *response.body_mut() = Body::from(serde_json::to_vec(
                    &api::get_mpn_account(Arc::clone(&context), serde_qs::from_str(&qs)?).await?,
                )?);
            }
            (Method::GET, "/peers") => {
                *response.body_mut() = Body::from(serde_json::to_vec(
                    &api::get_peers(Arc::clone(&context), serde_qs::from_str(&qs)?).await?,
                )?);
            }
            (Method::POST, "/peers") => {
                *response.body_mut() = Body::from(serde_json::to_vec(
                    &api::post_peer(Arc::clone(&context), serde_json::from_slice(&body_bytes)?)
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
                    &api::transact(Arc::clone(&context), bincode::deserialize(&body_bytes)?)
                        .await?,
                )?);
            }
            (Method::POST, "/bincode/transact/zero") => {
                *response.body_mut() = Body::from(bincode::serialize(
                    &api::transact_zero(Arc::clone(&context), bincode::deserialize(&body_bytes)?)
                        .await?,
                )?);
            }
            (Method::POST, "/bincode/transact/contract_payment") => {
                *response.body_mut() = Body::from(bincode::serialize(
                    &api::transact_contract_payment(
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
            (Method::GET, "/bincode/mempool/zero") => {
                if is_local {
                    *response.body_mut() = Body::from(bincode::serialize(
                        &api::get_zero_mempool(
                            Arc::clone(&context),
                            bincode::deserialize(&body_bytes)?,
                        )
                        .await?,
                    )?);
                } else {
                    *response.status_mut() = StatusCode::FORBIDDEN;
                }
            }
            (Method::GET, "/bincode/mempool") => {
                *response.body_mut() = Body::from(bincode::serialize(
                    &api::get_mempool(Arc::clone(&context), bincode::deserialize(&body_bytes)?)
                        .await?,
                )?);
            }
            _ => {
                *response.status_mut() = StatusCode::NOT_FOUND;
            }
        }

        if let Some(resp_sz) = response.body().size_hint().upper() {
            if let Some(client) = client {
                context
                    .write()
                    .await
                    .firewall
                    .add_traffic(client.ip(), resp_sz);
            }
        }

        Ok::<Response<Body>, NodeError>(response)
    }
    .await
    {
        Ok(resp) => Ok(resp),
        Err(e) => {
            if let Some(client) = client {
                let mut ctx = context.write().await;
                let default_punish = ctx.opts.default_punish;
                ctx.firewall.punish_bad(client.ip(), default_punish);
            }
            log::error!("Error: {}", e);
            Err(e)
        }
    }
}

use tokio::sync::mpsc;

pub async fn node_create<B: Blockchain>(
    opts: NodeOptions,
    network: &str,
    address: Option<PeerAddress>,
    priv_key: ed25519::PrivateKey,
    bootstrap: Vec<PeerAddress>,
    blockchain: B,
    timestamp_offset: i32,
    wallet: Option<Wallet>,
    social_profiles: SocialProfiles,
    mut incoming: mpsc::UnboundedReceiver<NodeRequest>,
    outgoing: mpsc::UnboundedSender<NodeRequest>,
) -> Result<(), NodeError> {
    let context = Arc::new(RwLock::new(NodeContext {
        firewall: Firewall::new(
            opts.ip_request_limit_per_minute,
            opts.traffic_limit_per_15m,
            opts.unresponsive_count_limit_per_15m,
        ),
        opts: opts.clone(),
        network: network.into(),
        social_profiles,
        address,
        pub_key: ed25519::PublicKey::from(priv_key.clone()),
        shutdown: false,
        outgoing: Arc::new(OutgoingSender {
            network: network.into(),
            chan: outgoing,
            priv_key,
        }),
        blockchain,
        wallet,
        mempool: HashMap::new(),
        zero_mempool: HashMap::new(),
        contract_payment_mempool: HashMap::new(),
        peers: bootstrap
            .into_iter()
            .map(|addr| {
                (
                    addr,
                    Peer {
                        pub_key: None,
                        address: addr,
                        info: None,
                    },
                )
            })
            .collect(),
        timestamp_offset,
        banned_headers: HashMap::new(),
        outdated_since: None,

        miner_puzzle: None,
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
