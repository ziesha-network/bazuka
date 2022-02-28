use super::{PeerAddress, PeerInfo, PeerStats};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Debug)]
pub struct PostPeerRequest {
    pub address: PeerAddress,
    pub info: PeerInfo,
    pub timestamp: u64,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PostPeerResponse {
    pub info: PeerInfo,
    pub timestamp: u64,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GetPeersRequest {}

#[derive(Deserialize, Serialize, Debug)]
pub struct GetPeersResponse {
    pub peers: HashMap<PeerAddress, PeerStats>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PostBlockRequest {}

#[derive(Deserialize, Serialize, Debug)]
pub struct PostBlockResponse {}

#[derive(Deserialize, Serialize, Debug)]
pub struct GetBlockRequest {
    pub since: usize,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GetBlockResponse {}
