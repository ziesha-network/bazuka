use crate::blockchain::ZkBlockchainPatch;
use crate::consensus::pow::Difficulty;
use crate::core::{
    Account, Amount, Block, ChainSourcedTx, ContractId, Header, Money, MpnDeposit, MpnSourcedTx,
    MpnWithdraw, TransactionAndDelta,
};
use crate::zk;
use std::collections::HashMap;
use thiserror::Error;

use super::{
    explorer::{ExplorerBlock, ExplorerMpnAccount},
    Peer, PeerAddress,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct SocialProfiles {
    pub discord: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetStatsRequest {}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetStatsResponse {
    pub social_profiles: SocialProfiles,
    pub address: String,
    pub height: u64,
    pub nodes: usize,
    pub power: u128,
    pub next_reward: Amount,
    pub timestamp: u32,
    pub version: String,
    pub network: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetAccountRequest {
    pub address: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetAccountResponse {
    pub account: Account,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetMpnAccountRequest {
    pub index: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetMpnAccountResponse {
    pub account: zk::MpnAccount,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetExplorerMpnAccountsRequest {
    pub page: usize,
    pub page_size: usize,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetExplorerMpnAccountsResponse {
    pub accounts: HashMap<u64, ExplorerMpnAccount>,
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
pub struct GetMinerPuzzleResponse {
    pub puzzle: Option<Puzzle>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Puzzle {
    pub key: String,        // Puzzle key encoded in hex
    pub blob: String,       // Blob encoded in hex
    pub offset: usize,      // From which byte the nonce starts?
    pub size: usize,        // How big is the nonce? (Bytes)
    pub target: Difficulty, // Difficulty target
    pub reward: Amount,     // Puzzle reward
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum HandshakeRequest {
    Node(PeerAddress),
    Client,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct HandshakeResponse {
    pub peer: Peer,
    pub timestamp: u32,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetPeersRequest {}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetPeersResponse {
    pub peers: Vec<PeerAddress>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostBlockRequest {
    pub block: Block,
    pub patch: ZkBlockchainPatch,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostBlockResponse {}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetBlocksRequest {
    pub since: u64,
    pub count: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetBlocksResponse {
    pub blocks: Vec<Block>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetExplorerBlocksRequest {
    pub since: u64,
    pub count: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetExplorerBlocksResponse {
    pub pow_hashes: Vec<String>,
    pub blocks: Vec<ExplorerBlock>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetOutdatedHeightsRequest {}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetOutdatedHeightsResponse {
    pub outdated_heights: HashMap<ContractId, u64>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetStatesRequest {
    pub outdated_heights: HashMap<ContractId, u64>,
    pub to: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetStatesResponse {
    pub patch: ZkBlockchainPatch,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetHeadersRequest {
    pub since: u64,
    pub count: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetHeadersResponse {
    pub headers: Vec<Header>,
    pub pow_keys: Vec<Vec<u8>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TransactRequest {
    pub tx_delta: TransactionAndDelta,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TransactResponse {}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostMpnTransactionRequest {
    pub tx: zk::MpnTransaction,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PostMpnTransactionResponse {}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ShutdownRequest {}

#[derive(Deserialize, Serialize, Debug)]
pub struct ShutdownResponse {}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetZeroMempoolRequest {}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetZeroMempoolResponse {
    pub updates: Vec<zk::MpnTransaction>,
    pub deposits: Vec<MpnDeposit>,
    pub withdraws: Vec<MpnWithdraw>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostMpnDepositRequest {
    pub tx: MpnDeposit,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PostMpnDepositResponse {}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostMpnWithdrawRequest {
    pub tx: MpnWithdraw,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PostMpnWithdrawResponse {}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetMempoolRequest {}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetMempoolResponse {
    pub chain_sourced: Vec<ChainSourcedTx>,
    pub mpn_sourced: Vec<MpnSourcedTx>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetDebugDataRequest {}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetDebugDataResponse {
    pub logs: String,
    pub db_checksum: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetBalanceRequest {
    pub address: String,
    pub token_id: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetBalanceResponse {
    pub balance: Amount,
    pub name: String,
    pub symbol: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct JsonMpnTransaction {
    pub nonce: u64,
    pub src_pub_key: String,
    pub dst_pub_key: String,

    pub src_token_index: u64,
    pub src_fee_token_index: u64,
    pub dst_token_index: u64,

    pub amount_token_id: String,
    pub amount: Amount,
    pub fee_token_id: String,
    pub fee: Amount,

    pub sig: String,
}

#[derive(Error, Debug)]
pub enum RequestDataError {
    #[error("invalid request data")]
    Invalid,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostJsonMpnTransactionRequest {
    pub tx: JsonMpnTransaction,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PostJsonMpnTransactionResponse {}

impl TryInto<PostMpnTransactionRequest> for PostJsonMpnTransactionRequest {
    type Error = RequestDataError;
    fn try_into(self) -> Result<PostMpnTransactionRequest, Self::Error> {
        Ok(PostMpnTransactionRequest {
            tx: zk::MpnTransaction {
                nonce: self.tx.nonce,
                src_pub_key: self
                    .tx
                    .src_pub_key
                    .parse()
                    .map_err(|_| Self::Error::Invalid)?,
                dst_pub_key: self
                    .tx
                    .dst_pub_key
                    .parse()
                    .map_err(|_| Self::Error::Invalid)?,

                src_token_index: self.tx.src_token_index,
                src_fee_token_index: self.tx.src_fee_token_index,
                dst_token_index: self.tx.dst_token_index,

                amount: Money {
                    token_id: self
                        .tx
                        .amount_token_id
                        .parse()
                        .map_err(|_| Self::Error::Invalid)?,
                    amount: self.tx.amount,
                },
                fee: Money {
                    token_id: self
                        .tx
                        .fee_token_id
                        .parse()
                        .map_err(|_| Self::Error::Invalid)?,
                    amount: self.tx.fee,
                },

                sig: bincode::deserialize(
                    &hex::decode(&self.tx.sig).map_err(|_| Self::Error::Invalid)?,
                )
                .map_err(|_| Self::Error::Invalid)?,
            },
        })
    }
}
