mod api;
mod context;
mod errors;
mod http;
use context::NodeContext;
pub use errors::NodeError;

use crate::blockchain::Blockchain;
use crate::utils;
use api::messages::*;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use std::net::SocketAddr;
use std::sync::Arc;

use crate::config::punish;

use serde_derive::{Deserialize, Serialize};

use futures::future::join_all;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use tokio::try_join;

pub type Timestamp = u64;

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PeerAddress(pub String, pub u16); // ip, port

impl std::fmt::Display for PeerAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "http://{}:{}", self.0, self.1)
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct PeerInfo {
    pub height: usize,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct PeerStats {
    pub punished_til: Option<Timestamp>,
    pub info: Option<PeerInfo>,
}

impl PeerStats {
    pub fn is_punished(&mut self) -> bool {
        let punished = match self.punished_til {
            Some(til) => utils::local_timestamp() < til,
            None => false,
        };
        if !punished {
            self.punished_til = None;
        }
        punished
    }
    pub fn punish(&mut self, millis: u64) {
        let now = utils::local_timestamp();
        self.punished_til = match self.punished_til {
            Some(curr) => Some(curr + millis),
            None => Some(now + millis),
        };
    }
}

pub struct Node<B: Blockchain> {
    address: PeerAddress,
    context: Arc<RwLock<NodeContext<B>>>,
}

async fn node_service<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: Request<Body>,
) -> Result<Response<Body>, NodeError> {
    let mut response = Response::new(Body::empty());
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let qs = req.uri().query().unwrap_or("").to_string();
    let body = req.into_body();

    match (method, &path[..]) {
        (Method::GET, "/peers") => {
            *response.body_mut() = Body::from(serde_json::to_vec(
                &api::get_peers(Arc::clone(&context), serde_qs::from_str(&qs).unwrap()).await?,
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
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
    };

    Ok(response)
}

impl<B: Blockchain + std::marker::Sync + std::marker::Send> Node<B> {
    pub fn new(address: PeerAddress, bootstrap: Vec<PeerAddress>, blockchain: B) -> Node<B> {
        Node {
            address,
            context: Arc::new(RwLock::new(NodeContext {
                blockchain,
                peers: bootstrap
                    .into_iter()
                    .map(|addr| {
                        (
                            addr,
                            PeerStats {
                                punished_til: None,
                                info: None,
                            },
                        )
                    })
                    .collect(),
                timestamp_offset: 0,
            })),
        }
    }

    async fn group_request<F, R>(
        &self,
        peers: &Vec<PeerAddress>,
        f: F,
    ) -> Vec<(PeerAddress, <R as futures::Future>::Output)>
    where
        F: Fn(PeerAddress) -> R,
        R: futures::Future,
    {
        peers
            .iter()
            .cloned()
            .zip(
                join_all(
                    peers
                        .iter()
                        .cloned()
                        .map(|peer| f(peer))
                        .collect::<Vec<_>>(),
                )
                .await
                .into_iter(),
            )
            .collect()
    }

    async fn heartbeat(&self) -> Result<(), NodeError> {
        loop {
            let ctx = self.context.read().await;
            let timestamp = ctx.network_timestamp();
            let info = ctx.get_info()?;
            let peer_addresses = ctx.peers.keys().cloned().collect::<Vec<PeerAddress>>();
            drop(ctx);

            let peer_responses: Vec<(PeerAddress, Result<PostPeerResponse, NodeError>)> = self
                .group_request(&peer_addresses, |peer| {
                    http::json_post::<PostPeerRequest, PostPeerResponse>(
                        format!("{}/peers", peer).to_string(),
                        PostPeerRequest {
                            address: self.address.clone(),
                            timestamp,
                            info: info.clone(),
                        },
                    )
                })
                .await;

            {
                let mut ctx = self.context.write().await;
                for bad_peer in peer_responses
                    .iter()
                    .filter(|(_, resp)| resp.is_err())
                    .map(|(p, _)| p)
                {
                    ctx.peers
                        .entry(bad_peer.clone())
                        .and_modify(|stats| stats.punish(punish::NO_RESPONSE_PUNISH));
                }
            }

            // Set timestamp_offset according to median timestamp of the network
            let median_timestamp = utils::median(
                &peer_responses
                    .iter()
                    .filter_map(|r| r.1.as_ref().ok())
                    .map(|r| r.timestamp)
                    .collect(),
            );
            let mut ctx = self.context.write().await;
            ctx.timestamp_offset = median_timestamp as i64 - utils::local_timestamp() as i64;
            drop(ctx);

            let mut ctx = self.context.write().await;
            let active_peers = ctx.active_peers().len();
            println!(
                "Lub dub! (Height: {}, Network Timestamp: {}, Peers: {})",
                ctx.blockchain.get_height()?,
                ctx.network_timestamp(),
                active_peers
            );

            sleep(Duration::from_millis(1000)).await;
        }
    }

    async fn server(&'static self) -> Result<(), NodeError> {
        let addr = SocketAddr::from(([127, 0, 0, 1], self.address.1));
        let node_context = self.context.clone();
        let make_svc = make_service_fn(|_| {
            let node_context = Arc::clone(&node_context);
            async move {
                Ok::<_, NodeError>(service_fn(move |req: Request<Body>| {
                    let node_context = Arc::clone(&node_context);
                    async move { node_service(node_context, req).await }
                }))
            }
        });
        Server::bind(&addr).serve(make_svc).await?;

        Ok(())
    }

    pub async fn run(&'static self) -> Result<(), NodeError> {
        let server_future = self.server();
        let heartbeat_future = self.heartbeat();

        try_join!(server_future, heartbeat_future)?;

        Ok(())
    }
}
