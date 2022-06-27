use super::*;

use super::api::messages::*;
use crate::blockchain::{BlockchainConfig, KvStoreChain};
use crate::config;
use crate::core::Signer;
use crate::crypto::SignatureScheme;
use crate::db::RamKvStore;
use crate::wallet::Wallet;

use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

struct Node {
    addr: PeerAddress,
    incoming: SenderWrapper,
    outgoing: mpsc::UnboundedReceiver<NodeRequest>,
}

pub struct NodeOpts {
    pub config: BlockchainConfig,
    pub priv_key: <Signer as SignatureScheme>::Priv,
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
    let (inc_send, inc_recv) = mpsc::unbounded_channel::<NodeRequest>();
    let (out_send, out_recv) = mpsc::unbounded_channel::<NodeRequest>();
    let node = node_create(
        config::node::get_test_node_options(),
        addr,
        opts.priv_key.clone(),
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
                sender: Arc::new(OutgoingSender {
                    chan: inc_send,
                    priv_key: opts.priv_key,
                }),
            },
            outgoing: out_recv,
        },
    )
}

async fn route(
    rules: Arc<RwLock<Vec<Rule>>>,
    src: PeerAddress,
    mut outgoing: mpsc::UnboundedReceiver<NodeRequest>,
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
        let inc_req = NodeRequest {
            socket_addr: None,
            body: req.body,
            resp: resp_snd,
        };
        if incs[&dst].sender.chan.send(inc_req).is_ok() {
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
    sender: Arc<OutgoingSender>,
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

    pub async fn outdated_states(&self) -> Result<GetOutdatedStatesResponse, NodeError> {
        self.sender
            .bincode_get::<GetOutdatedStatesRequest, GetOutdatedStatesResponse>(
                format!("{}/bincode/states/outdated", self.peer),
                GetOutdatedStatesRequest {},
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

    pub async fn mine(&self) -> Result<PostMinerSolutionResponse, NodeError> {
        let puzzle = self
            .sender
            .json_get::<GetMinerPuzzleRequest, Puzzle>(
                format!("{}/miner/puzzle", self.peer),
                GetMinerPuzzleRequest {},
                Limit::default(),
            )
            .await?;
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
