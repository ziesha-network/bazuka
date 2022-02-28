use crate::node::PeerAddress;

pub fn debug_bootstrap_nodes() -> Vec<PeerAddress> {
    vec![
        PeerAddress("127.0.0.1".to_string(), 3030),
        PeerAddress("127.0.0.1".to_string(), 3031),
        PeerAddress("127.0.0.1".to_string(), 3032),
        PeerAddress("127.0.0.1".to_string(), 3033),
        PeerAddress("127.0.0.1".to_string(), 3034),
        PeerAddress("127.0.0.1".to_string(), 3035),
    ]
}
