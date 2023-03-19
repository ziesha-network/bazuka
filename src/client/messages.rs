use crate::blockchain::{TransactionStats, ZkBlockchainPatch};
use crate::core::{
    Account, Address, Amount, Block, ChainSourcedTx, ContractId, Header, Money, MpnAddress,
    MpnDeposit, MpnSourcedTx, MpnWithdraw, Signature, Token, TransactionAndDelta, ValidatorProof,
};
use crate::mpn::MpnWork;
use crate::zk;
use std::collections::HashMap;
use thiserror::Error;

use super::{
    explorer::{
        ExplorerBlock, ExplorerChainSourcedTx, ExplorerMpnAccount, ExplorerMpnSourcedTx,
        ExplorerStaker,
    },
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
    pub next_reward: Amount,
    pub timestamp: u32,
    pub timestamp_offset: i32,
    pub epoch: u32,
    pub slot: u32,
    pub version: String,
    pub network: String,
    pub validator_claim: Option<ValidatorClaim>,
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
pub struct GetDelegationsRequest {
    pub address: String,
    pub top: usize,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetDelegationsResponse {
    pub delegators: HashMap<String, Amount>,
    pub delegatees: HashMap<String, Amount>,
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
pub enum HandshakeRequest {
    Node(PeerAddress),
    Client,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct HandshakeResponse {
    pub peer: Peer,
    pub timestamp: u32,
    pub timestamp_offset: i32,
    pub validator_claim: Option<ValidatorClaim>,
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
    pub blocks: Vec<ExplorerBlock>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetExplorerStakersRequest {}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetExplorerStakersResponse {
    pub current: Vec<ExplorerStaker>,
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
    pub height: u64,
    pub reward: Amount,
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
pub enum InputError {
    #[error("invalid input")]
    Invalid,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostJsonMpnTransactionRequest {
    pub tx: JsonMpnTransaction,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PostJsonMpnTransactionResponse {}

impl TryInto<PostMpnTransactionRequest> for PostJsonMpnTransactionRequest {
    type Error = InputError;
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

impl Into<GetJsonMempoolResponse> for GetMempoolResponse {
    fn into(self) -> GetJsonMempoolResponse {
        GetJsonMempoolResponse {
            updates: self
                .mpn_sourced
                .into_iter()
                .filter_map(|t| {
                    if let MpnSourcedTx::MpnTransaction(tx) = t {
                        Some(tx)
                    } else {
                        None
                    }
                })
                .map(|t| JsonMpnTransaction {
                    nonce: t.nonce,
                    src_pub_key: t.src_pub_key.to_string(),
                    dst_pub_key: t.dst_pub_key.to_string(),
                    src_token_index: t.src_token_index,
                    src_fee_token_index: t.src_fee_token_index,
                    dst_token_index: t.dst_token_index,
                    amount_token_id: t.amount.token_id.to_string(),
                    amount: t.amount.amount,
                    fee_token_id: t.fee.token_id.to_string(),
                    fee: t.fee.amount,
                    sig: hex::encode(bincode::serialize(&t.sig).unwrap()),
                })
                .collect(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetJsonMempoolRequest {
    pub mpn_address: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetJsonMempoolResponse {
    pub updates: Vec<JsonMpnTransaction>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetTokenInfoRequest {
    pub token_id: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetTokenInfoResponse {
    pub token: Token,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ValidatorClaim {
    pub timestamp: u32,
    pub address: Address,
    pub proof: ValidatorProof,
    pub node: PeerAddress,
    pub sig: Signature,
}

impl ValidatorClaim {
    pub fn verify_signature(&self) -> bool {
        use crate::crypto::SignatureScheme;
        match &self.sig {
            Signature::Unsigned => false,
            Signature::Signed(sig) => {
                let mut unsigned = self.clone();
                unsigned.sig = Signature::Unsigned;
                let bytes = bincode::serialize(&unsigned).unwrap();
                crate::core::Signer::verify(&self.address, &bytes, sig)
            }
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostValidatorClaimRequest {
    pub validator_claim: ValidatorClaim,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostValidatorClaimResponse {}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GenerateBlockRequest {}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GenerateBlockResponse {
    pub success: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetMpnWorkRequest {
    pub mpn_address: MpnAddress,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetMpnWorkResponse {
    pub works: HashMap<usize, MpnWork>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostMpnSolutionRequest {
    pub proofs: HashMap<usize, zk::groth16::Groth16Proof>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostMpnSolutionResponse {
    pub accepted: usize,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostMpnWorkerRequest {
    pub mpn_address: MpnAddress,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostMpnWorkerResponse {
    pub accepted: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetExplorerMempoolRequest {}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetExplorerMempoolResponse {
    pub chain_sourced: Vec<(ExplorerChainSourcedTx, TransactionStats)>,
    pub mpn_sourced: Vec<(ExplorerMpnSourcedTx, TransactionStats)>,
}
