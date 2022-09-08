use crate::client::{Peer, PeerAddress};
use rand::prelude::IteratorRandom;
use std::collections::HashMap;
use std::net::IpAddr;

struct CandidateDetails {
    address: PeerAddress,
    candidated_since: u32,
}

#[derive(Clone)]
struct PunishmentDetails {
    punished_till: u32,
}

pub struct PeerManager {
    candidate_remove_threshold: u32,
    self_addr: Option<PeerAddress>,
    candidates: HashMap<IpAddr, CandidateDetails>,
    punishments: HashMap<IpAddr, PunishmentDetails>,
    peers: HashMap<IpAddr, Peer>,
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
            peers: HashMap::new(),
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
        self.peers.remove(&ip);
        self.punishments.insert(
            ip,
            PunishmentDetails {
                punished_till: now + secs,
            },
        );
    }

    pub fn mark_as_candidate(&mut self, now: u32, addr: &PeerAddress) {
        if self.peers.contains_key(&addr.ip()) {
            self.peers.remove(&addr.ip());
            self.candidates.insert(
                addr.ip(),
                CandidateDetails {
                    address: *addr,
                    candidated_since: now,
                },
            );
        }
    }

    pub fn get_peers(&self) -> std::collections::hash_map::Values<'_, IpAddr, Peer> {
        self.peers.values()
    }

    pub fn random_candidates(&self, count: usize) -> Vec<PeerAddress> {
        self.candidates
            .values()
            .choose_multiple(&mut rand::thread_rng(), count)
            .into_iter()
            .map(|p| p.address)
            .collect()
    }

    pub fn random_peers(&self, count: usize) -> Vec<Peer> {
        self.get_peers()
            .choose_multiple(&mut rand::thread_rng(), count)
            .into_iter()
            .cloned()
            .collect()
    }

    pub fn add_candidate(&mut self, now: u32, addr: PeerAddress) {
        if self.self_addr == Some(addr) {
            return;
        }
        if !self.peers.contains_key(&addr.ip()) {
            self.candidates.insert(
                addr.ip(),
                CandidateDetails {
                    address: addr,
                    candidated_since: now,
                },
            );
        }
    }

    pub fn add_peer(&mut self, peer: Peer) {
        if self.self_addr == Some(peer.address) {
            return;
        }
        self.candidates.remove(&peer.address.ip());
        self.peers.insert(peer.address.ip(), peer);
    }
}
