use crate::blockchain::BlockchainError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NodeError {
    #[error("node not listening")]
    NotListeningError,
    #[error("node not answering")]
    NotAnsweringError,
    #[error("no peers found in the network")]
    NoPeers,
    #[error("blockchain error happened: {0}")]
    BlockchainError(#[from] BlockchainError),
    #[error("server error happened: {0}")]
    ServerError(#[from] hyper::Error),
    #[error("client error happened: {0}")]
    ClientError(#[from] hyper::http::Error),
    #[error("serde json error happened: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("serde qs error happened: {0}")]
    QueryStringError(#[from] serde_qs::Error),
    #[error("bincode error happened: {0}")]
    BincodeError(#[from] bincode::Error),
    #[error("utf8 error happened: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
    #[error("addr parse error happened: {0}")]
    AddrParseError(#[from] std::net::AddrParseError),
    #[error("no wallet available")]
    NoWalletError,
    #[error("no block is currently being mined")]
    NoCurrentlyMiningBlockError,
    #[error("timeout reached: {0}")]
    TimeoutError(#[from] tokio::time::error::Elapsed),
    #[error("http body size limit error")]
    SizeLimitError,
    #[error("bad input")]
    InputError,
}
