use std::result;

use thiserror::Error;

use crate::db::KvStoreError;

pub type Result<T> = result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("backend error: `{0}`")]
    UnderlyingBDErr(String),
    #[error("illegal format error: {0}")]
    IllegalFormat(String),
    #[error("kvstore error happened")]
    KvStoreError(#[from] KvStoreError),
}

pub fn with_db_error<T, F, E>(result: result::Result<T, E>, f: F) -> Result<T>
where
    F: FnOnce(dyn std::error::Error) -> Error,
    E: std::error::Error,
{
    match result {
        Ok(t) => Ok(t),
        Err(e) => Err(f(e)),
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}
