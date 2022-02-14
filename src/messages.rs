use serde_derive::{Deserialize, Serialize};

pub type PeerAddress = String;

#[derive(Deserialize, Serialize)]
pub struct PeerInfo {
    pub last_seen: u64,
    pub height: usize,
}

#[derive(Deserialize, Serialize)]
pub struct PostPeerRequest {
    pub address: PeerAddress,
    pub info: PeerInfo,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PostPeerResponse {}
