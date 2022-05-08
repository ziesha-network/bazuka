use super::*;

use crate::blockchain::KvStoreChain;
use crate::config::genesis;
use crate::db::RamKvStore;

struct Node {
    addr: PeerAddress,
    incoming: Arc<mpsc::UnboundedSender<IncomingRequest>>,
    outgoing: mpsc::UnboundedReceiver<OutgoingRequest>,
}

fn create_test_node(
    addr: PeerAddress,
    bootstrap: Vec<PeerAddress>,
) -> (impl futures::Future<Output = Result<(), NodeError>>, Node) {
    let chain = KvStoreChain::new(RamKvStore::new(), genesis::get_genesis_block()).unwrap();
    let (inc_send, inc_recv) = mpsc::unbounded_channel::<IncomingRequest>();
    let (out_send, out_recv) = mpsc::unbounded_channel::<OutgoingRequest>();
    let node = node_create(addr, bootstrap, chain, None, inc_recv, out_send);
    (
        node,
        Node {
            addr,
            incoming: Arc::new(inc_send),
            outgoing: out_recv,
        },
    )
}

async fn route(
    mut outgoing: mpsc::UnboundedReceiver<OutgoingRequest>,
    incs: HashMap<PeerAddress, Arc<mpsc::UnboundedSender<IncomingRequest>>>,
) -> Result<(), NodeError> {
    loop {
        if let Some(req) = outgoing.recv().await {
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
                .send(inc_req)
                .map_err(|_| NodeError::NotListeningError)?;
            req.resp
                .send(resp_rcv.recv().await.ok_or(NodeError::NotAnsweringError)?)
                .await
                .map_err(|_| NodeError::NotListeningError)?;
        } else {
            break;
        }
    }

    Ok(())
}

fn test_network(
    num_nodes: usize,
) -> (
    impl futures::Future,
    impl futures::Future,
    HashMap<PeerAddress, Arc<mpsc::UnboundedSender<IncomingRequest>>>,
) {
    let addresses: Vec<PeerAddress> = (0..num_nodes)
        .map(|i| PeerAddress(format!("127.0.0.1:{}", 3030 + i).parse().unwrap()))
        .collect();
    let (node_futs, nodes): (Vec<_>, Vec<Node>) = addresses
        .iter()
        .map(|p| create_test_node(*p, addresses.clone()))
        .unzip();
    let incs: HashMap<_, _> = nodes
        .iter()
        .map(|n| (n.addr, Arc::clone(&n.incoming)))
        .collect();
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
async fn test_node() {
    let (node_futs, route_futs, chans) = test_network(3);
    let test_logic = async {
        tokio::time::sleep(std::time::Duration::from_millis(5000)).await;
        println!("Start test logic...");
    };
    tokio::join!(node_futs, route_futs, test_logic);
}
