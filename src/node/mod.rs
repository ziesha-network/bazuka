mod api;
mod context;
mod errors;
mod http;
use context::NodeContext;
pub use errors::NodeError;

use crate::blockchain::Blockchain;
use api::messages::*;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use serde_derive::{Deserialize, Serialize};

use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use tokio::try_join;

pub type PeerAddress = String;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct PeerInfo {
    pub last_seen: u64,
    pub height: usize,
}

pub struct Node<B: Blockchain> {
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
    pub fn new(blockchain: B) -> Node<B> {
        Node {
            context: Arc::new(RwLock::new(NodeContext {
                blockchain,
                peers: HashMap::new(),
            })),
        }
    }

    async fn introduce(&self, addr: &str) -> Result<PostPeerResponse, NodeError> {
        let height = self.context.read().await.blockchain.get_height()?;
        Ok(http::json_post(
            &format!("{}/peers", addr),
            PostPeerRequest {
                address: "hahaha".to_string(),
                info: PeerInfo {
                    last_seen: 0,
                    height,
                },
            },
        )
        .await?)
    }

    async fn request_peers(&self, addr: &str) -> Result<Vec<String>, NodeError> {
        let resp: GetPeersResponse =
            http::json_get(&format!("{}/peers", addr), GetPeersRequest {}).await?;
        Ok(resp.peers.keys().cloned().collect())
    }

    async fn heartbeat(&self) -> Result<(), NodeError> {
        loop {
            let height = self.context.read().await.blockchain.get_height()?;
            println!("Lub dub! (Height: {})", height);
            sleep(Duration::from_millis(1000)).await;
            let peers = self.request_peers("http://127.0.0.1:3030").await?;
            println!("Peers: {:?}", peers);
            let result = self.introduce("http://127.0.0.1:3030").await?;
            println!("Introduced! Result: {:?}", result);
        }
    }

    async fn server(&'static self) -> Result<(), NodeError> {
        let addr = SocketAddr::from(([127, 0, 0, 1], 3030));
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
