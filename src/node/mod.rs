mod api;
mod context;
mod errors;
mod heartbeat;
mod http;
pub mod upnp;
use context::{NodeContext, TransactionStats};
pub use errors::NodeError;

#[cfg(feature = "pow")]
use context::Miner;

use thiserror::Error;
use crate::blockchain::Blockchain;
use crate::core::Block;
use crate::utils;
use crate::wallet::Wallet;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, Method, Request, Response, Server, StatusCode};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use crate::config::punish;

use serde_derive::{Deserialize, Serialize};

use hyper::server::conn::AddrStream;
use log::warn;
use tokio::sync::RwLock;
use tokio::try_join;
use crate::node::api::messages::PostBlockRequest;


#[derive(Error, Debug)]
pub enum PeerOperationError {
    #[error("announce block error")]
    AnnounceBlockError,
}


pub type Timestamp = u32;

#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PeerAddress(pub IpAddr, pub u16); // ip, port

impl std::fmt::Display for PeerAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "http://{}:{}", self.0, self.1)
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct PeerInfo {
    pub height: usize,
    #[cfg(feature = "pow")]
    pub power: u64,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct PeerStats {
    pub address: PeerAddress,
    pub punished_until: Timestamp,
    pub info: Option<PeerInfo>,
}

impl PeerStats {
    pub fn is_punished(&self) -> bool {
        utils::local_timestamp() < self.punished_until
    }
    pub fn punish(&mut self, secs: u32) {
        let now = utils::local_timestamp();
        self.punished_until = std::cmp::min(
            std::cmp::max(self.punished_until, now) + secs,
            now + punish::MAX_PUNISH,
        );
    }
    pub async fn announce_blocks(&self, blocks: &[Block]) -> Result<(), PeerOperationError> {
        let client = Client::new();

        for blk in blocks.iter() {
            let body = Body::from(
                bincode::serialize(&PostBlockRequest{block: blk.clone()})
                    .map_err(|_| PeerOperationError::AnnounceBlockError)?
            );
            let req = Request::builder()
                .method(Method::POST)
                .uri(format!("{}/bincode/blocks", self.address))
                .body(body).map_err(|_| PeerOperationError::AnnounceBlockError)?;
            let status = client
                .request(req)
                .await.map_err(|_| PeerOperationError::AnnounceBlockError)?
                .status();
            if status.as_u16() != 200 {
                warn!("can not announce blocks to peer: {}", self.address);
            }
        }

        return Ok(())
    }
}

pub struct Node<B: Blockchain> {
    address: PeerAddress,
    context: Arc<RwLock<NodeContext<B>>>,
}

async fn node_service<B: Blockchain>(
    _client: SocketAddr,
    context: Arc<RwLock<NodeContext<B>>>,
    req: Request<Body>,
) -> Result<Response<Body>, NodeError> {
    let mut response = Response::new(Body::empty());
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let qs = req.uri().query().unwrap_or("").to_string();
    let body = req.into_body();

    match (method, &path[..]) {
        // Miner will call this to fetch new PoW work.
        #[cfg(feature = "pow")]
        (Method::GET, "/miner/puzzle") => {
            *response.body_mut() = Body::from(serde_json::to_vec(
                &api::get_miner_puzzle(Arc::clone(&context), serde_qs::from_str(&qs)?).await?,
            )?);
        }

        // Miner will call this when he has solved the PoW puzzle.
        #[cfg(feature = "pow")]
        (Method::POST, "/miner/solution") => {
            *response.body_mut() = Body::from(serde_json::to_vec(
                &api::post_miner_solution(
                    Arc::clone(&context),
                    serde_json::from_slice(&hyper::body::to_bytes(body).await?)?,
                )
                .await?,
            )?);
        }

        // Register the miner software as a webhook.
        #[cfg(feature = "pow")]
        (Method::POST, "/miner") => {
            *response.body_mut() = Body::from(serde_json::to_vec(
                &api::post_miner(
                    Arc::clone(&context),
                    serde_json::from_slice(&hyper::body::to_bytes(body).await?)?,
                )
                .await?,
            )?);
        }

        (Method::GET, "/stats") => {
            *response.body_mut() = Body::from(serde_json::to_vec(
                &api::get_stats(Arc::clone(&context), serde_qs::from_str(&qs)?).await?,
            )?);
        }
        (Method::GET, "/peers") => {
            *response.body_mut() = Body::from(serde_json::to_vec(
                &api::get_peers(Arc::clone(&context), serde_qs::from_str(&qs)?).await?,
            )?);
        }
        (Method::POST, "/peers") => {
            *response.body_mut() = Body::from(serde_json::to_vec(
                &api::post_peer(
                    Arc::clone(&context),
                    serde_json::from_slice(&hyper::body::to_bytes(body).await?)?,
                )
                .await?,
            )?);
        }
        (Method::POST, "/bincode/transact") => {
            *response.body_mut() = Body::from(serde_json::to_vec(
                &api::transact(
                    Arc::clone(&context),
                    serde_json::from_slice(&hyper::body::to_bytes(body).await?)?,
                )
                .await?,
            )?);
        }
        (Method::GET, "/bincode/headers") => {
            *response.body_mut() = Body::from(bincode::serialize(
                &api::get_headers(Arc::clone(&context), serde_qs::from_str(&qs)?).await?,
            )?);
        }
        (Method::GET, "/bincode/blocks") => {
            *response.body_mut() = Body::from(bincode::serialize(
                &api::get_blocks(Arc::clone(&context), serde_qs::from_str(&qs)?).await?,
            )?);
        }
        (Method::POST, "/bincode/blocks") => {
            *response.body_mut() = Body::from(bincode::serialize(
                &api::post_block(
                    Arc::clone(&context),
                    bincode::deserialize(&hyper::body::to_bytes(body).await?)?,
                )
                .await?,
            )?);
        }
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
    };

    Ok(response)
}

impl<B: Blockchain + std::marker::Sync + std::marker::Send> Node<B> {
    pub fn new(
        address: PeerAddress,
        bootstrap: Vec<PeerAddress>,
        blockchain: B,
        wallet: Option<Wallet>,
    ) -> Node<B> {
        Node {
            address,
            context: Arc::new(RwLock::new(NodeContext {
                blockchain,
                wallet,
                mempool: HashMap::new(),
                peers: bootstrap
                    .into_iter()
                    .map(|addr| {
                        (
                            addr,
                            PeerStats {
                                address: addr,
                                punished_until: 0,
                                info: None,
                            },
                        )
                    })
                    .collect(),
                timestamp_offset: 0,
                #[cfg(feature = "pow")]
                miner: None,
            })),
        }
    }

    async fn server(&'static self) -> Result<(), NodeError> {
        let addr = SocketAddr::from(([0, 0, 0, 0], self.address.1));
        let node_context = self.context.clone();
        let make_svc = make_service_fn(|conn: &AddrStream| {
            let client = conn.remote_addr();
            let node_context = Arc::clone(&node_context);
            async move {
                Ok::<_, NodeError>(service_fn(move |req: Request<Body>| {
                    let node_context = Arc::clone(&node_context);
                    let client = client;
                    async move { node_service(client, node_context, req).await }
                }))
            }
        });
        Server::bind(&addr).serve(make_svc).await?;

        Ok(())
    }

    pub async fn run(&'static self) -> Result<(), NodeError> {
        let server_future = self.server();
        let heartbeat_future = heartbeat::heartbeater(self.address, Arc::clone(&self.context));

        try_join!(server_future, heartbeat_future)?;

        Ok(())
    }
}
