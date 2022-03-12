use crate::core::{Address, Block, Header, Money, Transaction};

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
pub struct PostBlockRequest {
    pub block: Block,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PostBlockResponse {}

#[derive(Deserialize, Serialize, Debug)]
pub struct GetBlocksRequest {
    pub since: usize,
    pub until: Option<usize>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GetBlocksResponse {
    pub blocks: Vec<Block>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GetHeadersRequest {
    pub since: usize,
    pub until: Option<usize>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GetHeadersResponse {
    pub headers: Vec<Header>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GetBalanceRequest {
    pub addr: Address,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GetBalanceResponse {
    pub amount: Money,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TransactRequest {
    pub tx: Transaction,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TransactResponse {}
