#[macro_use]
extern crate lazy_static;

pub mod blockchain;

#[cfg(not(tarpaulin_include))]
pub mod cli;

pub mod common;
pub mod config;
pub mod core;
pub mod crypto;
pub mod db;
pub mod mpn;
pub mod utils;
pub mod wallet;
pub mod zk;

#[cfg(feature = "node")]
pub mod node;

#[cfg(feature = "client")]
pub mod client;
