use std::time::{SystemTime, UNIX_EPOCH};

pub use decode::*;

mod decode;

pub fn local_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs()
}

pub fn median<T: Clone>(inps: &Vec<T>) -> T {
    inps[inps.len() / 2].clone()
}
