use super::messages::InputError;
use crate::blockchain::BlockchainError;
use crate::mpn::MpnError;
use crate::zk::ZkError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NodeError {
    #[error("node not listening")]
    NotListeningError,
    #[error("node not answering")]
    NotAnsweringError,
    #[error("node is run in client-only mode")]
    NodeIsClientOnly,
    #[error("blockchain error happened: {0}")]
    BlockchainError(#[from] BlockchainError),
    #[error("mpn error happened: {0}")]
    MpnError(#[from] MpnError),
    #[error("server error happened: {0}")]
    ServerError(#[from] hyper::Error),
    #[error("client error happened: {0}")]
    ClientError(#[from] hyper::http::Error),
    #[error("http invalid header error happened: {0}")]
    InvalidHeaderError(#[from] hyper::header::InvalidHeaderValue),
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
    #[error("cannot parse account address: {0}")]
    AccountParseAddressError(#[from] crate::core::ParseAddressError),
    #[error("cannot parse mpn address: {0}")]
    MpnAccountParseAddressError(#[from] crate::core::ParseMpnAddressError),
    #[error("cannot parse account address: {0}")]
    TokenIdParseError(#[from] crate::core::ParseTokenIdError),
    #[error("timeout reached: {0}")]
    TimeoutError(#[from] tokio::time::error::Elapsed),
    #[error("http body size limit error")]
    SizeLimitError,
    #[error("bad input: {0}")]
    InputError(#[from] InputError),
    #[error("signature (authorization) header is invalid")]
    InvalidSignatureHeader,
    #[error("signature required on this message")]
    SignatureRequired,
    #[error("zk error: {0}")]
    ZkError(#[from] ZkError),
    #[error("wrong network")]
    WrongNetwork,
    #[error("states are outdated")]
    StatesOutdated,
    #[error("requester ip is different with proposed peer")]
    HandshakeClientMismatch,
    #[error("remote server error: {0}")]
    RemoteServerError(String),
    #[error("block timestamp is way higher than current network timestamp")]
    BlockTimestampInFuture,
    #[error("your validator is not exposed on the internet")]
    ValidatorNotExposed,
    #[error("request sender's ip address is unknown")]
    SenderIpUnknown,
}
