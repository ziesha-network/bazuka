use crate::client::{Peer, PeerAddress};
use crate::utils;
use std::collections::HashMap;

pub struct PeerManager {
    candidates: HashMap<PeerAddress, u32>,  // Candidate since?
    punishments: HashMap<PeerAddress, u32>, // Punished until?
    peers: HashMap<PeerAddress, Peer>,
}

impl PeerManager {
    pub fn new(bootstrap: Vec<PeerAddress>, now: u32) -> Self {
        Self {
            candidates: bootstrap.into_iter().map(|b| (b, now)).collect(),
            punishments: HashMap::new(),
            peers: HashMap::new(),
        }
    }

    pub fn refresh(&mut self, now: u32) {
        // Mark punished peers as candidates after the punishment time has ended
        for (peer, punished_till) in self.punishments.clone().into_iter() {
            if now > punished_till {
                self.punishments.remove(&peer);
                self.mark_as_candidate(now, &peer);
            }
        }

        // Remove candidates that are older than a certain time
        self.candidates
            .retain(|_, candidate_since| (now - *candidate_since) < 600); // TODO: Remove hardcoded number
    }

    pub fn is_punished(&self, now: u32, peer: &PeerAddress) -> bool {
        self.punishments
            .get(peer)
            .map(|till| now < *till)
            .unwrap_or(false)
    }

    // Punish peer for a certain time
    pub fn punish_peer(&mut self, now: u32, peer: &PeerAddress) {
        self.candidates.remove(peer);
        self.peers.remove(peer);
        self.punishments.insert(*peer, now + 3600);
    }

    pub fn mark_as_candidate(&mut self, now: u32, addr: &PeerAddress) {
        if self.peers.contains_key(&addr) {
            self.peers.remove(&addr);
            self.candidates.insert(*addr, utils::local_timestamp());
        }
    }

    pub fn get_peers(&self) -> &HashMap<PeerAddress, Peer> {
        &self.peers
    }

    pub fn add_candidate(&mut self, now: u32, addr: PeerAddress) {
        if !self.peers.contains_key(&addr) {
            self.candidates.insert(addr, utils::local_timestamp());
        }
    }

    pub fn add_peer(&mut self, addr: PeerAddress, peer: Peer) {
        self.candidates.remove(&addr);
        self.peers.insert(addr, peer);
    }
}
