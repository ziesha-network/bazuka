use crate::core::{Address, ContractPayment, Signer, TransactionAndDelta};
use crate::crypto::ed25519;
use crate::crypto::SignatureScheme;
use crate::utils;
use crate::zk::ZeroTransaction;
use hyper::body::HttpBody;
use hyper::header::{HeaderValue, AUTHORIZATION};
use hyper::{Body, Method, Request, Response};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;

mod error;
pub mod messages;
pub use error::NodeError;
use messages::*;

pub type Timestamp = u32;

const NETWORK_HEADER: &str = "NETWORK";

#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PeerAddress(pub SocketAddr); // ip, port

impl std::fmt::Display for PeerAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "http://{}", self.0)
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct PeerInfo {
    pub height: u64,
    pub power: u128,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Peer {
    pub pub_key: Option<ed25519::PublicKey>,
    pub address: PeerAddress,
    pub punished_until: Timestamp,
    pub info: Option<PeerInfo>,
}

impl Peer {
    pub fn is_punished(&self) -> bool {
        utils::local_timestamp() < self.punished_until
    }
    pub fn punish(&mut self, secs: u32, max_punish: u32) {
        let now = utils::local_timestamp();
        self.punished_until = std::cmp::min(
            std::cmp::max(self.punished_until, now) + secs,
            now + max_punish,
        );
    }
}

pub struct NodeRequest {
    pub socket_addr: Option<SocketAddr>,
    pub body: Request<Body>,
    pub resp: mpsc::Sender<Result<Response<Body>, NodeError>>,
}

pub struct OutgoingSender {
    pub priv_key: ed25519::PrivateKey,
    pub network: String,
    pub chan: mpsc::UnboundedSender<NodeRequest>,
}

#[derive(Default)]
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
    pub async fn raw(&self, mut body: Request<Body>, limit: Limit) -> Result<Body, NodeError> {
        let (resp_snd, mut resp_rcv) = mpsc::channel::<Result<Response<Body>, NodeError>>(1);
        body.headers_mut()
            .insert(NETWORK_HEADER, HeaderValue::from_str(&self.network)?);
        let req = NodeRequest {
            socket_addr: None,
            body,
            resp: resp_snd,
        };
        self.chan
            .send(req)
            .map_err(|_| NodeError::NotListeningError)?;

        let body = if let Some(time_limit) = limit.time {
            timeout(time_limit, resp_rcv.recv()).await?
        } else {
            resp_rcv.recv().await
        }
        .ok_or(NodeError::NotAnsweringError)??
        .into_body();

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

        Ok(body)
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
            AUTHORIZATION,
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
        let resp: Resp = bincode::deserialize(&hyper::body::to_bytes(body).await?)?;
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
        let resp: Resp = bincode::deserialize(&hyper::body::to_bytes(body).await?)?;
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
        let resp: Resp = serde_json::from_slice(&hyper::body::to_bytes(body).await?)?;
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
        let resp: Resp = serde_json::from_slice(&hyper::body::to_bytes(body).await?)?;
        Ok(resp)
    }
}

#[derive(Clone)]
pub struct BazukaClient {
    pub peer: PeerAddress,
    pub sender: Arc<OutgoingSender>,
}

impl BazukaClient {
    pub fn connect(
        priv_key: ed25519::PrivateKey,
        peer: PeerAddress,
        network: String,
    ) -> (impl futures::Future<Output = Result<(), NodeError>>, Self) {
        let (sender_send, mut sender_recv) = mpsc::unbounded_channel::<NodeRequest>();
        let client_loop = async move {
            while let Some(req) = sender_recv.recv().await {
                let resp = async {
                    let client = hyper::Client::new();
                    let resp = client.request(req.body).await?;
                    Ok::<_, NodeError>(resp)
                }
                .await;
                if let Err(e) = req.resp.send(resp).await {
                    log::error!("Node not listening to its HTTP request answer: {}", e);
                }
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
            },
        )
    }
    pub async fn shutdown(&self) -> Result<(), NodeError> {
        self.sender
            .json_post::<ShutdownRequest, ShutdownResponse>(
                format!("{}/shutdown", self.peer),
                ShutdownRequest {},
                Limit::default(),
            )
            .await?;
        Ok(())
    }
    pub async fn stats(&self) -> Result<GetStatsResponse, NodeError> {
        self.sender
            .json_get::<GetStatsRequest, GetStatsResponse>(
                format!("{}/stats", self.peer),
                GetStatsRequest {},
                Limit::default(),
            )
            .await
    }
    pub async fn peers(&self) -> Result<GetPeersResponse, NodeError> {
        self.sender
            .json_get::<GetPeersRequest, GetPeersResponse>(
                format!("{}/peers", self.peer),
                GetPeersRequest {},
                Limit::default(),
            )
            .await
    }

    pub async fn get_zero_mempool(&self) -> Result<GetZeroMempoolResponse, NodeError> {
        self.sender
            .bincode_get::<GetZeroMempoolRequest, GetZeroMempoolResponse>(
                format!("{}/bincode/mempool/zero", self.peer),
                GetZeroMempoolRequest {},
                Limit::default(),
            )
            .await
    }

    pub async fn transact_contract_payment(
        &self,
        tx: ContractPayment,
    ) -> Result<TransactContractPaymentResponse, NodeError> {
        self.sender
            .bincode_post::<TransactContractPaymentRequest, TransactContractPaymentResponse>(
                format!("{}/bincode/transact/contract_payment", self.peer),
                TransactContractPaymentRequest { tx },
                Limit::default(),
            )
            .await
    }

    pub async fn outdated_heights(&self) -> Result<GetOutdatedHeightsResponse, NodeError> {
        self.sender
            .bincode_get::<GetOutdatedHeightsRequest, GetOutdatedHeightsResponse>(
                format!("{}/bincode/states/outdated", self.peer),
                GetOutdatedHeightsRequest {},
                Limit::default(),
            )
            .await
    }

    pub async fn get_account(&self, address: Address) -> Result<GetAccountResponse, NodeError> {
        self.sender
            .json_get::<GetAccountRequest, GetAccountResponse>(
                format!("{}/account", self.peer),
                GetAccountRequest {
                    address: address.to_string(),
                },
                Limit::default(),
            )
            .await
    }

    pub async fn get_mpn_account(&self, index: u32) -> Result<GetMpnAccountResponse, NodeError> {
        self.sender
            .json_get::<GetMpnAccountRequest, GetMpnAccountResponse>(
                format!("{}/mpn/account", self.peer),
                GetMpnAccountRequest { index },
                Limit::default(),
            )
            .await
    }

    pub async fn transact(
        &self,
        tx_delta: TransactionAndDelta,
    ) -> Result<TransactResponse, NodeError> {
        self.sender
            .bincode_post::<TransactRequest, TransactResponse>(
                format!("{}/bincode/transact", self.peer),
                TransactRequest { tx_delta },
                Limit::default(),
            )
            .await
    }

    pub async fn zero_transact(
        &self,
        tx: ZeroTransaction,
    ) -> Result<TransactZeroResponse, NodeError> {
        self.sender
            .bincode_post::<TransactZeroRequest, TransactZeroResponse>(
                format!("{}/bincode/transact/zero", self.peer),
                TransactZeroRequest { tx },
                Limit::default(),
            )
            .await
    }

    pub async fn mine(&self) -> Result<PostMinerSolutionResponse, NodeError> {
        let puzzle = self
            .sender
            .json_get::<GetMinerPuzzleRequest, GetMinerPuzzleResponse>(
                format!("{}/miner/puzzle", self.peer),
                GetMinerPuzzleRequest {},
                Limit::default(),
            )
            .await?
            .puzzle
            .unwrap(); // TODO: Don't unwrap
        let sol = mine_puzzle(&puzzle);
        self.sender
            .json_post::<PostMinerSolutionRequest, PostMinerSolutionResponse>(
                format!("{}/miner/solution", self.peer),
                sol,
                Limit::default(),
            )
            .await
    }
}

fn mine_puzzle(puzzle: &Puzzle) -> PostMinerSolutionRequest {
    let key = hex::decode(&puzzle.key).unwrap();
    let mut blob = hex::decode(&puzzle.blob).unwrap();
    let mut nonce = 0u64;
    loop {
        blob[puzzle.offset..puzzle.offset + puzzle.size].copy_from_slice(&nonce.to_le_bytes());
        let hash = crate::consensus::pow::hash(&key, &blob);
        if hash.meets_difficulty(rust_randomx::Difficulty::new(puzzle.target)) {
            return PostMinerSolutionRequest {
                nonce: hex::encode(nonce.to_le_bytes()),
            };
        }

        nonce += 1;
    }
}
