use super::*;

use super::api::messages::*;
use crate::blockchain::KvStoreChain;
use crate::config::genesis;
use crate::db::RamKvStore;
use rand::Rng;
use std::time::Duration;
use tokio::time::sleep;

struct Node {
    addr: PeerAddress,
    incoming: SenderWrapper,
    outgoing: mpsc::UnboundedReceiver<OutgoingRequest>,
}

fn create_test_node(
    addr: PeerAddress,
    bootstrap: Vec<PeerAddress>,
) -> (impl futures::Future<Output = Result<(), NodeError>>, Node) {
    let timestamp_offset = rand::thread_rng().gen_range(-100..100);
    let chain = KvStoreChain::new(RamKvStore::new(), genesis::get_genesis_block()).unwrap();
    let (inc_send, inc_recv) = mpsc::unbounded_channel::<IncomingRequest>();
    let (out_send, out_recv) = mpsc::unbounded_channel::<OutgoingRequest>();
    let node = node_create(
        addr,
        bootstrap,
        chain,
        timestamp_offset,
        None,
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
    mut outgoing: mpsc::UnboundedReceiver<OutgoingRequest>,
    incs: HashMap<PeerAddress, SenderWrapper>,
) -> Result<(), NodeError> {
    while let Some(req) = outgoing.recv().await {
        let s = PeerAddress(
            req.body
                .uri()
                .authority()
                .unwrap()
                .to_string()
                .parse()
                .unwrap(),
        );
        let (resp_snd, mut resp_rcv) = mpsc::channel::<Result<Response<Body>, NodeError>>(1);
        let inc_req = IncomingRequest {
            socket_addr: s.0,
            body: req.body,
            resp: resp_snd,
        };
        incs[&s]
            .chan
            .send(inc_req)
            .map_err(|_| NodeError::NotListeningError)?;
        req.resp
            .send(resp_rcv.recv().await.ok_or(NodeError::NotAnsweringError)?)
            .await
            .map_err(|_| NodeError::NotListeningError)?;
    }

    Ok(())
}

#[derive(Clone)]
struct SenderWrapper {
    peer: PeerAddress,
    chan: Arc<mpsc::UnboundedSender<IncomingRequest>>,
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
    async fn json_get<Req: serde::Serialize, Resp: serde::de::DeserializeOwned>(
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
    async fn json_post<Req: serde::Serialize, Resp: serde::de::DeserializeOwned>(
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
    async fn shutdown(&self) -> Result<(), NodeError> {
        self.json_post::<ShutdownRequest, ShutdownResponse>("shutdown", ShutdownRequest {})
            .await?;
        Ok(())
    }
    async fn stats(&self) -> Result<GetStatsResponse, NodeError> {
        self.json_get::<GetStatsRequest, GetStatsResponse>("stats", GetStatsRequest {})
            .await
    }
}

fn test_network(
    num_nodes: usize,
) -> (
    impl futures::Future,
    impl futures::Future,
    HashMap<PeerAddress, SenderWrapper>,
) {
    let addresses: Vec<PeerAddress> = (0..num_nodes)
        .map(|i| PeerAddress(format!("127.0.0.1:{}", 3030 + i).parse().unwrap()))
        .collect();
    let (node_futs, nodes): (Vec<_>, Vec<Node>) = addresses
        .iter()
        .map(|p| create_test_node(*p, addresses.clone()))
        .unzip();
    let incs: HashMap<_, _> = nodes.iter().map(|n| (n.addr, n.incoming.clone())).collect();
    let route_futs = nodes
        .into_iter()
        .map(|n| route(n.outgoing, incs.clone()))
        .collect::<Vec<_>>();

    (
        futures::future::join_all(node_futs),
        futures::future::join_all(route_futs),
        incs,
    )
}

#[tokio::test]
async fn test_timestamps_are_sync() {
    let (node_futs, route_futs, chans) = test_network(3);
    let test_logic = async {
        sleep(Duration::from_millis(2000)).await;

        let mut timestamps = Vec::new();
        for chan in chans.values() {
            timestamps.push(chan.stats().await.unwrap().timestamp);
        }
        let first = timestamps.first().unwrap();
        assert!(timestamps.iter().all(|t| t == first));

        for chan in chans.values() {
            chan.shutdown().await.unwrap();
        }
    };
    tokio::join!(node_futs, route_futs, test_logic);
}
