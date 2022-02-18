use super::blockchain::{Blockchain, BlockchainError};
use super::messages::*;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, Method, Request, Response, Server, StatusCode};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use tokio::try_join;

#[derive(Error, Debug)]
pub enum NodeError {
    #[error("blockchain error happened")]
    BlockchainError(#[from] BlockchainError),
    #[error("server error happened")]
    ServerError(#[from] hyper::Error),
    #[error("client error happened")]
    ClientError(#[from] hyper::http::Error),
    #[error("serde error happened")]
    SerdeError(#[from] serde_json::Error),
}

pub struct NodeContext<B: Blockchain> {
    blockchain: B,
    peers: HashMap<PeerAddress, PeerInfo>,
}

pub struct Node<B: Blockchain> {
    context: Arc<RwLock<NodeContext<B>>>,
}

async fn json_post<Req: serde::Serialize, Resp: serde::de::DeserializeOwned>(
    addr: &str,
    req: Req,
) -> Result<Resp, NodeError> {
    let client = Client::new();
    let req = Request::builder()
        .method(Method::POST)
        .uri(addr)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&req)?))?;
    let body = client.request(req).await?.into_body();
    let resp: Resp = serde_json::from_slice(&hyper::body::to_bytes(body).await?)?;
    Ok(resp)
}

async fn json_get<Resp: serde::de::DeserializeOwned>(addr: &str) -> Result<Resp, NodeError> {
    let client = Client::new();
    let req = Request::builder()
        .method(Method::GET)
        .uri(addr)
        .body(Body::empty())?;
    let body = client.request(req).await?.into_body();
    let resp: Resp = serde_json::from_slice(&hyper::body::to_bytes(body).await?)?;
    Ok(resp)
}

async fn node_service<B: Blockchain>(
    context: Arc<RwLock<NodeContext<B>>>,
    req: Request<Body>,
) -> Result<Response<Body>, NodeError> {
    let mut response = Response::new(Body::empty());
    let mut context = context.write().await;

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/peers") => {
            *response.body_mut() = Body::from(serde_json::to_vec(&GetPeersResponse {
                peers: context.peers.clone(),
            })?);
        }
        (&Method::POST, "/peers") => {
            let body = req.into_body();
            let req: PostPeerRequest = serde_json::from_slice(&hyper::body::to_bytes(body).await?)?;
            context.peers.insert(req.address, req.info);
            *response.body_mut() = Body::from(serde_json::to_vec(&PostPeerResponse {})?);
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
        json_post(
            &format!("{}/peers", addr),
            PostPeerRequest {
                address: "hahaha".to_string(),
                info: PeerInfo {
                    last_seen: 0,
                    height,
                },
            },
        )
        .await
    }

    async fn request_peers(&self, addr: &str) -> Result<Vec<String>, NodeError> {
        let resp: GetPeersResponse = json_get(&format!("{}/peers", addr)).await?;
        Ok(resp.peers.keys().cloned().collect())
    }

    async fn heartbeat(&self) -> Result<(), NodeError> {
        loop {
            println!("Lub dub!");
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
