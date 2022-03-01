use crate::node::PeerAddress;

pub fn debug_bootstrap_nodes() -> Vec<PeerAddress> {
    vec![
        PeerAddress("127.0.0.1".parse().unwrap(), 3030),
        PeerAddress("127.0.0.1".parse().unwrap(), 3031),
        PeerAddress("127.0.0.1".parse().unwrap(), 3032),
        PeerAddress("127.0.0.1".parse().unwrap(), 3033),
        PeerAddress("127.0.0.1".parse().unwrap(), 3034),
        PeerAddress("127.0.0.1".parse().unwrap(), 3035),
    ]
}
