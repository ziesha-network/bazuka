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

// TODO: Move to consensus folder
pub fn calc_pow_difficulty(
    diff_calc_interval: u64,
    block_time: usize,
    last_pow: &ProofOfWork,
    prev_pow: &ProofOfWork,
) -> crate::consensus::pow::Difficulty {
    let time_delta = last_pow.timestamp - prev_pow.timestamp;
    let avg_block_time = time_delta / (diff_calc_interval - 1) as u32;
    let diff_change = (block_time as f32 / avg_block_time as f32).clamp(0.5f32, 2f32);
    let new_diff = rust_randomx::Difficulty::new(last_pow.target.0).scale(diff_change);
    crate::consensus::pow::Difficulty(new_diff.to_u32())
}
