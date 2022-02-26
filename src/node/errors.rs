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
}
