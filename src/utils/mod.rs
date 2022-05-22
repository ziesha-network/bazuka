use crate::config;
use crate::core::ProofOfWork;
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

pub fn calc_pow_difficulty(last_pow: &ProofOfWork, prev_pow: &ProofOfWork) -> u32 {
    let time_delta = last_pow.timestamp - prev_pow.timestamp;
    let avg_block_time = time_delta / (config::DIFFICULTY_CALC_INTERVAL - 1) as u32;
    let diff_change = (config::BLOCK_TIME as f32 / avg_block_time as f32).clamp(0.5f32, 2f32);
    let new_diff = rust_randomx::Difficulty::new(last_pow.target).scale(diff_change);
    new_diff.to_u32()
}
