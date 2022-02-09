use std::fmt::{Debug, Display, Formatter};

mod auth;
mod babe;
mod epochs;
mod forktree;
mod header;
mod slots;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, derive_more::Display)]
pub enum Error {
    #[display(fmt = "bad type of slot")]
    BadBabeSlotType,
    #[display(fmt = "multiple pre-runtime digest, reject!")]
    MultiplePreRuntimeDigests,
    #[display(fmt = "no pre-runtime digest found")]
    NoPreRuntimeDigests,
    #[display(fmt = "bad digest format {:?}", _0)]
    BadDigestFormat(dyn std::error::Error),
}

impl std::error::Error for Error {}
