use super::*;

use crate::blockchain::KvStoreChain;
use crate::config::genesis;
use crate::db::RamKvStore;

async fn handle_connections(
    peers: HashMap<
        PeerAddress,
        (
            mpsc::UnboundedSender<IncomingRequest>,
            mpsc::UnboundedReceiver<OutgoingRequest>,
        ),
    >,
) {
}

fn create_test_node(
    addr: PeerAddress,
) -> (
    impl futures::Future,
    (
        mpsc::UnboundedSender<IncomingRequest>,
        mpsc::UnboundedReceiver<OutgoingRequest>,
    ),
) {
    let chain = KvStoreChain::new(RamKvStore::new(), genesis::get_genesis_block()).unwrap();
    let (inc_send, inc_recv) = mpsc::unbounded_channel::<IncomingRequest>();
    let (out_send, out_recv) = mpsc::unbounded_channel::<OutgoingRequest>();
    let node = node_create(addr, Vec::new(), chain, None, inc_recv, out_send);
    (node, (inc_send, out_recv))
}

#[tokio::test]
async fn test_node() {
    let addr1 = PeerAddress("127.0.0.1:3031".parse().unwrap());
    let addr2 = PeerAddress("127.0.0.1:3032".parse().unwrap());
    let (node1, chans1) = create_test_node(addr1);
    let (node2, chans2) = create_test_node(addr2);
    let conns = handle_connections([(addr1, chans1), (addr2, chans2)].into_iter().collect());
    tokio::join!(node1, node2);
}
