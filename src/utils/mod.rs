use std::time::{SystemTime, UNIX_EPOCH};

pub fn local_timestamp() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() as u32
}

pub fn median<T: Clone + std::cmp::Ord>(inps: &[T]) -> Option<T> {
    let mut sorted = inps.to_vec();
    sorted.sort();
    if sorted.len() > 0 {
        Some(sorted[sorted.len() / 2].clone())
    } else {
        None
    }
}
