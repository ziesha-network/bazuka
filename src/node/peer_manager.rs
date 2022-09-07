use crate::client::{Peer, PeerAddress};
use std::collections::{HashMap, HashSet};

pub struct PeerManager {
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
    pub fn remove_peer(&mut self, addr: &PeerAddress) {
        self.peers.remove(&addr);
        self.candidates.remove(&addr);
    }
    pub fn addresses(&self) -> Vec<PeerAddress> {
        self.candidates
            .iter()
            .chain(self.peers.keys())
            .cloned()
            .collect()
    }
    pub fn get_peers(&self) -> &HashMap<PeerAddress, Peer> {
        &self.peers
    }
    pub fn add_candidate(&mut self, addr: PeerAddress) {
        if !self.peers.contains_key(&addr) {
            self.candidates.insert(addr);
        }
    }
    pub fn add_peer(&mut self, addr: PeerAddress, peer: Peer) {
        self.candidates.remove(&addr);
        self.peers.insert(addr, peer);
    }
}
