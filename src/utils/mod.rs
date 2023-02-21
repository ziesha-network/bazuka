use std::time::{SystemTime, UNIX_EPOCH};

pub fn local_timestamp() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() as u32
}

pub fn median<T: Clone + std::cmp::Ord>(inps: &[T]) -> T {
    let mut sorted = inps.to_vec();
    sorted.sort();
    sorted[sorted.len() / 2].clone()
}
