use crate::client::{PeerAddress, PeerInfo};
use crate::crypto::ed25519;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct Peer {
    pub pub_key: ed25519::PublicKey,
    pub address: PeerAddress,
    pub info: PeerInfo,
}

struct PeerManager {
    candidates: HashSet<PeerAddress>,
    peers: HashMap<PeerAddress, Peer>,
}

impl PeerManager {
    pub fn new(bootstrap: Vec<PeerAddress>) -> Self {
        Self {
            candidates: bootstrap.into_iter().collect(),
            peers: HashMap::new(),
        }
    }
}
