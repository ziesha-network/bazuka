use crate::core::{Address, Block, Header, Money, Transaction};

use super::{PeerAddress, PeerInfo, PeerStats};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "pow")]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostMinerSolutionRequest {
    pub nonce: String,
}

#[cfg(feature = "pow")]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostMinerSolutionResponse {}

#[cfg(feature = "pow")]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetMinerPuzzleRequest {}

#[cfg(feature = "pow")]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Puzzle {
    pub key: String,   // Puzzle key encoded in hex
    pub blob: String,  // Blob encoded in hex
    pub offset: usize, // From which byte the nonce starts?
    pub size: usize,   // How big is the nonce? (Bytes)
    pub target: u32,   // Difficulty target
}

#[cfg(feature = "pow")]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RegisterMinerRequest {
    pub webhook: String,
}

#[cfg(feature = "pow")]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RegisterMinerResponse {}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostPeerRequest {
    pub address: PeerAddress,
    pub info: PeerInfo,
    pub timestamp: u32,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostPeerResponse {
    pub info: PeerInfo,
    pub timestamp: u32,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetPeersRequest {}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetPeersResponse {
    pub peers: HashMap<PeerAddress, PeerStats>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostBlockRequest {
    pub block: Block,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostBlockResponse {}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetBlocksRequest {
    pub since: usize,
    pub until: Option<usize>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetBlocksResponse {
    pub blocks: Vec<Block>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetHeadersRequest {
    pub since: usize,
    pub until: Option<usize>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetHeadersResponse {
    pub headers: Vec<Header>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetBalanceRequest {
    pub addr: Address,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetBalanceResponse {
    pub amount: Money,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TransactRequest {
    pub tx: Transaction,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TransactResponse {}
