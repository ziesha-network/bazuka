use crate::blockchain::BlockchainError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NodeError {
    #[error("blockchain error happened")]
    BlockchainError(#[from] BlockchainError),
    #[error("server error happened")]
    ServerError(#[from] hyper::Error),
    #[error("client error happened")]
    ClientError(#[from] hyper::http::Error),
    #[error("serde json error happened")]
    JsonError(#[from] serde_json::Error),
    #[error("serde qs error happened")]
    QueryStringError(#[from] serde_qs::Error),
    #[error("bincode error happened")]
    BincodeError(#[from] bincode::Error),
    #[error("utf8 error happened")]
    Utf8Error(#[from] std::str::Utf8Error),
    #[error("addr parse error happened")]
    AddrParseError(#[from] std::net::AddrParseError),
    #[error("no wallet available")]
    NoWalletError,
    #[error("no miner is registered")]
    NoMinerError,
    #[error("no block is currently being mined")]
    NoCurrentlyMiningBlockError,
}
