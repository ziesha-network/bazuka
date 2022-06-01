use crate::blockchain::ZkBlockchainPatch;
use crate::core::{Address, Block, ContractId, Header, Money, TransactionAndDelta};
use crate::zk;
use std::collections::HashMap;

use super::{Peer, PeerAddress, PeerInfo};
use serde_derive::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetStatsRequest {}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetStatsResponse {
    pub height: u64,
    pub power: u128,
    pub next_reward: Money,
    pub timestamp: u32,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostMinerSolutionRequest {
    pub nonce: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostMinerSolutionResponse {}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetMinerPuzzleRequest {}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Puzzle {
    pub key: String,   // Puzzle key encoded in hex
    pub blob: String,  // Blob encoded in hex
    pub offset: usize, // From which byte the nonce starts?
    pub size: usize,   // How big is the nonce? (Bytes)
    pub target: u32,   // Difficulty target
}

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
    pub peers: Vec<Peer>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostBlockRequest {
    pub block: Block,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostBlockResponse {}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetBlocksRequest {
    pub since: u64,
    pub until: Option<u64>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetBlocksResponse {
    pub blocks: Vec<Block>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetStatesRequest {
    pub outdated_states: HashMap<ContractId, zk::ZkCompressedState>,
    pub to: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetStatesResponse {
    pub patch: ZkBlockchainPatch,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetHeadersRequest {
    pub since: u64,
    pub until: Option<u64>,
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
    pub tx_delta: TransactionAndDelta,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TransactResponse {}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TransactZeroRequest {
    pub tx: zk::ZeroTransaction,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TransactZeroResponse {}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ShutdownRequest {}

#[derive(Deserialize, Serialize, Debug)]
pub struct ShutdownResponse {}
