#[macro_use]
extern crate lazy_static;

use std::sync::{Arc, Mutex};

const MAX_LOG_CAP: usize = 1000;

lazy_static! {
    static ref GLOBAL_LOGS: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
}

pub fn report_log(log: &str) {
    let mut logs = GLOBAL_LOGS.lock().unwrap();
    logs.push(log.into());
    while logs.len() > MAX_LOG_CAP {
        logs.remove(0);
    }
}

pub mod blockchain;

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
