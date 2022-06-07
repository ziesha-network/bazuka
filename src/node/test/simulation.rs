use super::*;

use super::api::messages::*;
use crate::blockchain::{BlockchainConfig, KvStoreChain};
use crate::config;
use crate::db::RamKvStore;
use crate::wallet::Wallet;

use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

struct Node {
    addr: PeerAddress,
    incoming: SenderWrapper,
    outgoing: mpsc::UnboundedReceiver<OutgoingRequest>,
}

pub struct NodeOpts {
    pub config: BlockchainConfig,
    pub wallet: Option<Wallet>,
    pub addr: u16,
    pub bootstrap: Vec<u16>,
    pub timestamp_offset: i32,
}

fn create_test_node(
    opts: NodeOpts,
) -> (impl futures::Future<Output = Result<(), NodeError>>, Node) {
    let addr = PeerAddress(SocketAddr::from(([127, 0, 0, 1], opts.addr)));
    let chain = KvStoreChain::new(RamKvStore::new(), opts.config).unwrap();
    let (inc_send, inc_recv) = mpsc::unbounded_channel::<IncomingRequest>();
    let (out_send, out_recv) = mpsc::unbounded_channel::<OutgoingRequest>();
    let node = node_create(
        config::node::get_test_node_options(),
        addr,
        opts.bootstrap
            .iter()
            .map(|p| PeerAddress(SocketAddr::from(([127, 0, 0, 1], *p))))
            .collect(),
        chain,
        opts.timestamp_offset,
        opts.wallet,
        inc_recv,
        out_send,
    );
    (
        node,
        Node {
            addr,
            incoming: SenderWrapper {
                peer: addr,
                chan: Arc::new(inc_send),
            },
            outgoing: out_recv,
        },
    )
}

async fn route(
    rules: Arc<RwLock<Vec<Rule>>>,
    src: PeerAddress,
    mut outgoing: mpsc::UnboundedReceiver<OutgoingRequest>,
    incs: HashMap<PeerAddress, SenderWrapper>,
) -> Result<(), NodeError> {
    while let Some(req) = outgoing.recv().await {
        let rules = rules.read().await.clone();
        let mut dst = PeerAddress(
            req.body
                .uri()
                .authority()
                .unwrap()
                .to_string()
                .parse()
                .unwrap(),
        );
        let rule = rules.iter().find(|r| r.applies(&req.body, src, dst));

        if let Some(rule) = rule {
            match rule.action {
                Action::Drop => {
                    continue;
                }
                Action::Delay(dur) => {
                    sleep(dur).await;
                }
                Action::Redirect(port) => {
                    dst = PeerAddress(SocketAddr::from(([127, 0, 0, 1], port)));
                }
            }
        }

        let (resp_snd, mut resp_rcv) = mpsc::channel::<Result<Response<Body>, NodeError>>(1);
        let inc_req = IncomingRequest {
            socket_addr: dst.0,
            body: req.body,
            resp: resp_snd,
        };
        if incs[&dst].chan.send(inc_req).is_ok() {
            if let Some(answer) = resp_rcv.recv().await {
                let _ = req.resp.send(answer).await;
            }
        }
    }

    Ok(())
}

#[derive(Clone)]
pub struct SenderWrapper {
    peer: PeerAddress,
    chan: Arc<mpsc::UnboundedSender<IncomingRequest>>,
}

#[derive(Clone)]
pub enum Action {
    Drop,
    Delay(Duration),
    Redirect(u16),
}

#[derive(Clone)]
pub enum Endpoint {
    Any,
    Peer(u16),
}

#[derive(Clone)]
pub struct Rule {
    pub from: Endpoint,
    pub to: Endpoint,
    pub url: String,
    pub action: Action,
}

impl Rule {
    pub fn drop_all() -> Self {
        Rule {
            from: Endpoint::Any,
            to: Endpoint::Any,
            url: "".into(),
            action: Action::Drop,
        }
    }
    pub fn drop_url(url: &str) -> Self {
        Rule {
            from: Endpoint::Any,
            to: Endpoint::Any,
            url: url.into(),
            action: Action::Drop,
        }
    }
}

impl Rule {
    fn applies(&self, req: &Request<Body>, req_from: PeerAddress, req_to: PeerAddress) -> bool {
        req.uri().to_string().contains(&self.url)
            && match self.from {
                Endpoint::Any => true,
                Endpoint::Peer(port) => req_from.0.port() == port,
            }
            && match self.to {
                Endpoint::Any => true,
                Endpoint::Peer(port) => req_to.0.port() == port,
            }
    }
}

impl SenderWrapper {
    pub async fn raw(&self, body: Request<Body>) -> Result<Body, NodeError> {
        let (resp_snd, mut resp_rcv) = mpsc::channel::<Result<Response<Body>, NodeError>>(1);
        let req = IncomingRequest {
            socket_addr: "0.0.0.0:0".parse().unwrap(),
            body,
            resp: resp_snd,
        };
        self.chan
            .send(req)
            .map_err(|_| NodeError::NotListeningError)?;

        let body = resp_rcv
            .recv()
            .await
            .ok_or(NodeError::NotAnsweringError)??
            .into_body();

        Ok(body)
    }
    #[allow(dead_code)]
    pub async fn json_get<Req: serde::Serialize, Resp: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        req: Req,
    ) -> Result<Resp, NodeError> {
        let req = Request::builder()
            .method(Method::GET)
            .uri(format!(
                "{}/{}?{}",
                self.peer,
                url,
                serde_qs::to_string(&req)?
            ))
            .body(Body::empty())?;
        let body = self.raw(req).await?;
        let resp: Resp = serde_json::from_slice(&hyper::body::to_bytes(body).await?)?;
        Ok(resp)
    }
    pub async fn json_post<Req: serde::Serialize, Resp: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        req: Req,
    ) -> Result<Resp, NodeError> {
        let req = Request::builder()
            .method(Method::POST)
            .uri(format!("{}/{}", self.peer, url))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&req)?))?;
        let body = self.raw(req).await?;
        let resp: Resp = serde_json::from_slice(&hyper::body::to_bytes(body).await?)?;
        Ok(resp)
    }
    pub async fn bincode_get<Req: serde::Serialize, Resp: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        req: Req,
    ) -> Result<Resp, NodeError> {
        let req = Request::builder()
            .method(Method::GET)
            .uri(format!("{}/{}", self.peer, url,))
            .body(Body::from(bincode::serialize(&req)?))?;
        let body = self.raw(req).await?;
        let resp: Resp = bincode::deserialize(&hyper::body::to_bytes(body).await?)?;
        Ok(resp)
    }
    pub async fn bincode_post<Req: serde::Serialize, Resp: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        req: Req,
    ) -> Result<Resp, NodeError> {
        let req = Request::builder()
            .method(Method::POST)
            .uri(format!("{}/{}", self.peer, url))
            .header("content-type", "application/octet-stream")
            .body(Body::from(bincode::serialize(&req)?))?;
        let body = self.raw(req).await?;
        let resp: Resp = bincode::deserialize(&hyper::body::to_bytes(body).await?)?;
        Ok(resp)
    }
    pub async fn shutdown(&self) -> Result<(), NodeError> {
        self.json_post::<ShutdownRequest, ShutdownResponse>("shutdown", ShutdownRequest {})
            .await?;
        Ok(())
    }
    pub async fn stats(&self) -> Result<GetStatsResponse, NodeError> {
        self.json_get::<GetStatsRequest, GetStatsResponse>("stats", GetStatsRequest {})
            .await
    }
    pub async fn peers(&self) -> Result<GetPeersResponse, NodeError> {
        self.json_get::<GetPeersRequest, GetPeersResponse>("peers", GetPeersRequest {})
            .await
    }

    pub async fn outdated_states(&self) -> Result<GetOutdatedStatesResponse, NodeError> {
        self.bincode_get::<GetOutdatedStatesRequest, GetOutdatedStatesResponse>(
            "bincode/states/outdated",
            GetOutdatedStatesRequest {},
        )
        .await
    }

    pub async fn transact(
        &self,
        tx_delta: TransactionAndDelta,
    ) -> Result<TransactResponse, NodeError> {
        self.bincode_post::<TransactRequest, TransactResponse>(
            "bincode/transact",
            TransactRequest { tx_delta },
        )
        .await
    }

    pub async fn mine(&self) -> Result<PostMinerSolutionResponse, NodeError> {
        let puzzle = self
            .json_get::<GetMinerPuzzleRequest, Puzzle>("miner/puzzle", GetMinerPuzzleRequest {})
            .await?;
        let sol = mine_puzzle(&puzzle);
        self.json_post::<PostMinerSolutionRequest, PostMinerSolutionResponse>("miner/solution", sol)
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

pub fn test_network(
    rules: Arc<RwLock<Vec<Rule>>>,
    node_opts: Vec<NodeOpts>,
) -> (
    impl futures::Future<Output = Result<Vec<()>, NodeError>>,
    impl futures::Future<Output = Result<Vec<()>, NodeError>>,
    Vec<SenderWrapper>,
) {
    let (node_futs, nodes): (Vec<_>, Vec<Node>) = node_opts
        .into_iter()
        .map(|node_opts| create_test_node(node_opts))
        .unzip();
    let incs: HashMap<_, _> = nodes.iter().map(|n| (n.addr, n.incoming.clone())).collect();
    let route_futs = nodes
        .into_iter()
        .map(|n| route(Arc::clone(&rules), n.addr, n.outgoing, incs.clone()))
        .collect::<Vec<_>>();

    (
        futures::future::try_join_all(node_futs),
        futures::future::try_join_all(route_futs),
        incs.into_values().collect(),
    )
}
