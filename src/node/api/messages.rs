use super::{PeerAddress, PeerInfo};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize)]
pub struct PostPeerRequest {
    pub address: PeerAddress,
    pub info: PeerInfo,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PostPeerResponse {}

#[derive(Deserialize, Serialize)]
pub struct GetPeersRequest {}

#[derive(Deserialize, Serialize)]
pub struct GetPeersResponse {
    pub peers: HashMap<PeerAddress, PeerInfo>,
}

#[derive(Deserialize, Serialize)]
pub struct PostBlockRequest {}

#[derive(Deserialize, Serialize)]
pub struct PostBlockResponse {}

#[derive(Deserialize, Serialize)]
pub struct GetBlockRequest {
    pub since: usize,
}

#[derive(Deserialize, Serialize)]
pub struct GetBlockResponse {}
