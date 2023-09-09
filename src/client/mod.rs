use crate::blockchain::TransactionMetadata;
use crate::core::{Address, ContractId, GeneralTransaction, MpnAddress, Signer};
use crate::crypto::ed25519;
use crate::crypto::SignatureScheme;
use crate::zk::ZkProof;
use hyper::body::{Bytes, HttpBody};
use hyper::header::HeaderValue;
use hyper::{Body, Method, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;

mod error;
pub mod explorer;
pub mod messages;
pub mod utils;
pub use error::NodeError;
use messages::*;

pub type Timestamp = u32;

pub const SIGNATURE_HEADER: &str = "X-ZIESHA-SIGNATURE";
pub const NETWORK_HEADER: &str = "X-ZIESHA-NETWORK-NAME";

#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PeerAddress(pub SocketAddr); // ip, port

impl PeerAddress {
    pub fn ip(&self) -> IpAddr {
        self.0.ip()
    }
}

impl std::fmt::Display for PeerAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for PeerAddress {
    type Err = std::net::AddrParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        SocketAddr::from_str(s).map(Self)
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Peer {
    pub address: PeerAddress,
    pub pub_key: ed25519::PublicKey,
    pub height: u64,
    pub power: f64,
}

pub struct NodeRequest {
    pub limit: Limit,
    pub socket_addr: Option<SocketAddr>,
    pub body: Request<Body>,
    pub resp: mpsc::UnboundedSender<Result<Response<Body>, NodeError>>,
}

pub struct OutgoingSender {
    pub priv_key: ed25519::PrivateKey,
    pub network: String,
    pub chan: mpsc::UnboundedSender<NodeRequest>,
}

#[derive(Default, Clone)]
pub struct Limit {
    pub time: Option<Duration>,
    pub size: Option<u64>,
}

impl Limit {
    pub fn time(mut self, time: u32) -> Self {
        self.time = Some(Duration::from_millis(time as u64));
        self
    }
    pub fn size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }
}

impl OutgoingSender {
    pub async fn raw(&self, mut body: Request<Body>, limit: Limit) -> Result<Bytes, NodeError> {
        let (resp_snd, mut resp_rcv) =
            mpsc::unbounded_channel::<Result<Response<Body>, NodeError>>();
        body.headers_mut()
            .insert(NETWORK_HEADER, HeaderValue::from_str(&self.network)?);
        let req = NodeRequest {
            limit: limit.clone(),
            socket_addr: None,
            body,
            resp: resp_snd,
        };
        self.chan
            .send(req)
            .map_err(|_| NodeError::NotListeningError)?;

        let recver = async {
            let resp = resp_rcv
                .recv()
                .await
                .ok_or(NodeError::NotAnsweringError)??;
            let status = resp.status();
            let body = resp.into_body();

            if let Some(size_limit) = limit.size {
                if body
                    .size_hint()
                    .upper()
                    .map(|u| u > size_limit)
                    .unwrap_or(true)
                {
                    return Err(NodeError::SizeLimitError);
                }
            }
            Ok::<_, NodeError>((status, hyper::body::to_bytes(body).await?))
        };

        let (status, body_bytes) = if let Some(time_limit) = limit.time {
            timeout(time_limit, recver).await?
        } else {
            recver.await
        }?;

        if status != StatusCode::OK {
            return Err(NodeError::RemoteServerError(
                String::from_utf8_lossy(&body_bytes).to_string(),
            ));
        }

        Ok(body_bytes)
    }

    fn sign(
        &self,
        req: hyper::http::request::Builder,
        body: Vec<u8>,
    ) -> Result<Request<Body>, NodeError> {
        let pub_key = hex::encode(bincode::serialize(&ed25519::PublicKey::from(
            self.priv_key.clone(),
        ))?);
        let sig = hex::encode(bincode::serialize(&Signer::sign(&self.priv_key, &body))?);
        let mut req = req.body(Body::from(body))?;
        req.headers_mut().insert(
            SIGNATURE_HEADER,
            HeaderValue::from_str(&format!("{}-{}", pub_key, sig))?,
        );
        Ok(req)
    }

    pub async fn bincode_get<Req: serde::Serialize, Resp: serde::de::DeserializeOwned>(
        &self,
        addr: String,
        req: Req,
        limit: Limit,
    ) -> Result<Resp, NodeError> {
        let bytes = bincode::serialize(&req)?;
        let req = self.sign(Request::builder().method(Method::GET).uri(&addr), bytes)?;
        let body = self.raw(req, limit).await?;
        let resp: Resp = bincode::deserialize(&body)?;
        Ok(resp)
    }

    #[allow(dead_code)]
    pub async fn bincode_post<Req: serde::Serialize, Resp: serde::de::DeserializeOwned>(
        &self,
        addr: String,
        req: Req,
        limit: Limit,
    ) -> Result<Resp, NodeError> {
        let bytes = bincode::serialize(&req)?;
        let req = self.sign(
            Request::builder()
                .method(Method::POST)
                .uri(&addr)
                .header("content-type", "application/octet-stream"),
            bytes,
        )?;
        let body = self.raw(req, limit).await?;
        let resp: Resp = bincode::deserialize(&body)?;
        Ok(resp)
    }

    pub async fn json_post<Req: serde::Serialize, Resp: serde::de::DeserializeOwned>(
        &self,
        addr: String,
        req: Req,
        limit: Limit,
    ) -> Result<Resp, NodeError> {
        let bytes = serde_json::to_vec(&req)?;

        let req = self.sign(
            Request::builder()
                .method(Method::POST)
                .uri(&addr)
                .header("content-type", "application/json"),
            bytes,
        )?;

        let body = self.raw(req, limit).await?;
        let resp: Resp = serde_json::from_slice(&body)?;
        Ok(resp)
    }

    pub async fn json_get<Req: serde::Serialize, Resp: serde::de::DeserializeOwned>(
        &self,
        addr: String,
        req: Req,
        limit: Limit,
    ) -> Result<Resp, NodeError> {
        let req = self.sign(
            Request::builder().method(Method::GET).uri(format!(
                "{}?{}",
                addr,
                serde_qs::to_string(&req)?
            )),
            vec![],
        )?;
        let body = self.raw(req, limit).await?;
        let resp: Resp = serde_json::from_slice(&body)?;
        Ok(resp)
    }
}

#[derive(Clone)]
pub struct BazukaClient {
    pub peer: PeerAddress,
    pub sender: Arc<OutgoingSender>,
    pub limit: Option<Limit>,
}

impl BazukaClient {
    pub fn connect(
        priv_key: ed25519::PrivateKey,
        peer: PeerAddress,
        network: String,
        limit: Option<Limit>,
    ) -> (impl futures::Future<Output = Result<(), NodeError>>, Self) {
        let (sender_send, mut sender_recv) = mpsc::unbounded_channel::<NodeRequest>();
        let client_loop = async move {
            while let Some(req) = sender_recv.recv().await {
                tokio::task::spawn(async move {
                    let resp = async {
                        let client = hyper::Client::new();
                        let resp = client.request(req.body).await?;
                        Ok::<_, NodeError>(resp)
                    }
                    .await;
                    if let Err(e) = req.resp.send(resp) {
                        log::error!("Node not listening to its HTTP request answer: {}", e);
                    }
                });
            }
            Ok::<(), NodeError>(())
        };
        (
            client_loop,
            Self {
                peer,
                sender: Arc::new(OutgoingSender {
                    priv_key,
                    network,
                    chan: sender_send,
                }),
                limit,
            },
        )
    }
    pub async fn shutdown(&self) -> Result<(), NodeError> {
        self.sender
            .json_post::<ShutdownRequest, ShutdownResponse>(
                format!("http://{}/shutdown", self.peer),
                ShutdownRequest {},
                self.limit.clone().unwrap_or_default(),
            )
            .await?;
        Ok(())
    }
    pub async fn stats(&self) -> Result<GetStatsResponse, NodeError> {
        self.sender
            .json_get::<GetStatsRequest, GetStatsResponse>(
                format!("http://{}/stats", self.peer),
                GetStatsRequest {},
                self.limit.clone().unwrap_or_default(),
            )
            .await
    }
    pub async fn peers(&self) -> Result<GetPeersResponse, NodeError> {
        self.sender
            .json_get::<GetPeersRequest, GetPeersResponse>(
                format!("http://{}/peers", self.peer),
                GetPeersRequest {},
                self.limit.clone().unwrap_or_default(),
            )
            .await
    }

    pub async fn get_headers(
        &self,
        since: u64,
        count: u64,
    ) -> Result<GetHeadersResponse, NodeError> {
        self.sender
            .bincode_get::<GetHeadersRequest, GetHeadersResponse>(
                format!("http://{}/bincode/headers", self.peer),
                GetHeadersRequest { since, count },
                self.limit.clone().unwrap_or_default(),
            )
            .await
    }

    pub async fn get_blocks(&self, since: u64, count: u64) -> Result<GetBlocksResponse, NodeError> {
        self.sender
            .bincode_get::<GetBlocksRequest, GetBlocksResponse>(
                format!("http://{}/bincode/blocks", self.peer),
                GetBlocksRequest { since, count },
                self.limit.clone().unwrap_or_default(),
            )
            .await
    }

    pub async fn get_account(&self, address: Address) -> Result<GetAccountResponse, NodeError> {
        self.sender
            .json_get::<GetAccountRequest, GetAccountResponse>(
                format!("http://{}/account", self.peer),
                GetAccountRequest {
                    address: address.to_string(),
                },
                self.limit.clone().unwrap_or_default(),
            )
            .await
    }

    pub async fn get_delegations(
        &self,
        address: Address,
        top: usize,
    ) -> Result<GetDelegationsResponse, NodeError> {
        self.sender
            .json_get::<GetDelegationsRequest, GetDelegationsResponse>(
                format!("http://{}/delegations", self.peer),
                GetDelegationsRequest {
                    address: address.to_string(),
                    top,
                },
                self.limit.clone().unwrap_or_default(),
            )
            .await
    }

    pub async fn get_balance(
        &self,
        address: Address,
        token_id: ContractId,
    ) -> Result<GetBalanceResponse, NodeError> {
        self.sender
            .json_get::<GetBalanceRequest, GetBalanceResponse>(
                format!("http://{}/balance", self.peer),
                GetBalanceRequest {
                    address: address.to_string(),
                    token_id: token_id.to_string(),
                },
                self.limit.clone().unwrap_or_default(),
            )
            .await
    }

    pub async fn get_token(&self, token_id: ContractId) -> Result<GetTokenInfoResponse, NodeError> {
        self.sender
            .json_get::<GetTokenInfoRequest, GetTokenInfoResponse>(
                format!("http://{}/token", self.peer),
                GetTokenInfoRequest {
                    token_id: token_id.to_string(),
                },
                self.limit.clone().unwrap_or_default(),
            )
            .await
    }

    pub async fn get_mpn_account(
        &self,
        mpn_address: MpnAddress,
    ) -> Result<GetMpnAccountResponse, NodeError> {
        self.sender
            .json_get::<GetMpnAccountRequest, GetMpnAccountResponse>(
                format!("http://{}/mpn/account", self.peer),
                GetMpnAccountRequest {
                    address: mpn_address.to_string(),
                },
                self.limit.clone().unwrap_or_default(),
            )
            .await
    }

    pub async fn transact(&self, tx: GeneralTransaction) -> Result<TransactResponse, NodeError> {
        self.sender
            .bincode_post::<TransactRequest, TransactResponse>(
                format!("http://{}/bincode/transact", self.peer),
                TransactRequest {
                    tx,
                    meta: Some(TransactionMetadata {
                        claimed_timestamp: crate::utils::local_timestamp(),
                    }),
                },
                self.limit.clone().unwrap_or_default(),
            )
            .await
    }

    pub async fn mine(&self) -> Result<GenerateBlockResponse, NodeError> {
        self.sender
            .bincode_post::<GenerateBlockRequest, GenerateBlockResponse>(
                format!("http://{}/generate_block", self.peer),
                GenerateBlockRequest {},
                self.limit.clone().unwrap_or_default(),
            )
            .await
    }

    pub async fn get_mpn_works(&self, address: Address) -> Result<GetMpnWorkResponse, NodeError> {
        self.sender
            .bincode_get::<GetMpnWorkRequest, GetMpnWorkResponse>(
                format!("http://{}/bincode/mpn/work", self.peer),
                GetMpnWorkRequest { address },
                self.limit.clone().unwrap_or_default(),
            )
            .await
    }

    pub async fn post_mpn_worker(
        &self,
        address: Address,
    ) -> Result<PostMpnWorkerResponse, NodeError> {
        self.sender
            .bincode_post::<PostMpnWorkerRequest, PostMpnWorkerResponse>(
                format!("http://{}/bincode/mpn/worker", self.peer),
                PostMpnWorkerRequest { address },
                self.limit.clone().unwrap_or_default(),
            )
            .await
    }

    pub async fn post_mpn_proof(
        &self,
        prover: Address,
        proofs: HashMap<usize, ZkProof>,
    ) -> Result<PostMpnSolutionResponse, NodeError> {
        self.sender
            .bincode_post::<PostMpnSolutionRequest, PostMpnSolutionResponse>(
                format!("http://{}/bincode/mpn/solution", self.peer),
                PostMpnSolutionRequest { prover, proofs },
                self.limit.clone().unwrap_or_default(),
            )
            .await
    }
}
