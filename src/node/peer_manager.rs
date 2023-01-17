use crate::client::{Peer, PeerAddress};
use rand::prelude::IteratorRandom;
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Duration;

struct CandidateDetails {
    address: PeerAddress,
    candidated_since: u32,
}

#[derive(Clone)]
struct NodeDetails {
    peer: Peer,
    ping_time: Duration,
}

#[derive(Clone)]
struct PunishmentDetails {
    punished_till: u32,
}

pub struct PeerManager {
    candidate_remove_threshold: u32,
    self_addr: Option<PeerAddress>,
    candidates: HashMap<IpAddr, CandidateDetails>,
    nodes: HashMap<IpAddr, NodeDetails>,
    punishments: HashMap<IpAddr, PunishmentDetails>,
    peers: Vec<Peer>,
}

impl PeerManager {
    pub fn new(
        self_addr: Option<PeerAddress>,
        bootstrap: Vec<PeerAddress>,
        now: u32,
        candidate_remove_threshold: u32,
    ) -> Self {
        Self {
            candidate_remove_threshold,
            self_addr,
            candidates: bootstrap
                .into_iter()
                .map(|b| {
                    (
                        b.ip(),
                        CandidateDetails {
                            address: b,
                            candidated_since: now,
                        },
                    )
                })
                .collect(),
            punishments: HashMap::new(),
            nodes: HashMap::new(),
            peers: Vec::new(),
        }
    }

    pub fn refresh(&mut self, now: u32) {
        // Mark punished peers as candidates after the punishment time has ended
        for (ip, punish_details) in self.punishments.clone().into_iter() {
            if now > punish_details.punished_till {
                self.punishments.remove(&ip);
            }
        }

        // Remove candidates that are older than a certain time
        self.candidates
            .retain(|_, det| (now - det.candidated_since) < self.candidate_remove_threshold);
    }

    pub fn is_ip_punished(&self, now: u32, ip: IpAddr) -> bool {
        self.punishments
            .get(&ip)
            .map(|det| now < det.punished_till)
            .unwrap_or(false)
    }

    // Punish peer for a certain time
    pub fn punish_ip_for(&mut self, now: u32, ip: IpAddr, secs: u32) {
        self.candidates.remove(&ip);
        self.nodes.remove(&ip);
        self.punishments.insert(
            ip,
            PunishmentDetails {
                punished_till: now + secs,
            },
        );
    }

    pub fn mark_as_candidate(&mut self, now: u32, addr: &PeerAddress) {
        if self.nodes.contains_key(&addr.ip()) {
            self.nodes.remove(&addr.ip());
            self.candidates.insert(
                addr.ip(),
                CandidateDetails {
                    address: *addr,
                    candidated_since: now,
                },
            );
        }
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn get_nodes(&self) -> impl Iterator<Item = &Peer> {
        self.nodes.values().map(|n| &n.peer)
    }

    pub fn random_candidates(&self, count: usize) -> Vec<PeerAddress> {
        self.candidates
            .values()
            .choose_multiple(&mut rand::thread_rng(), count)
            .into_iter()
            .map(|p| p.address)
            .collect()
    }

    pub fn select_peers(&mut self, count: usize) {
        let mut vals = self.nodes.values().cloned().collect::<Vec<_>>();
        vals.sort_unstable_by_key(|d| d.ping_time);
        self.peers = vals.into_iter().take(count).map(|d| d.peer).collect();
    }

    pub fn get_peers(&self) -> Vec<Peer> {
        self.peers.clone()
    }

    pub fn add_candidate(&mut self, now: u32, addr: PeerAddress) {
        if self.self_addr == Some(addr) {
            return;
        }
        if !self.nodes.contains_key(&addr.ip()) {
            self.candidates.insert(
                addr.ip(),
                CandidateDetails {
                    address: addr,
                    candidated_since: now,
                },
            );
        }
    }

    pub fn add_node(&mut self, peer: Peer, ping_time: Duration) {
        if self.self_addr == Some(peer.address) {
            return;
        }
        self.candidates.remove(&peer.address.ip());
        self.nodes
            .insert(peer.address.ip(), NodeDetails { peer, ping_time });
    }
}
