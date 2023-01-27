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
    timestamps: &[u32],
    block_time: u32,
    difficulty_window: u64,
    difficulty_cut: u64,
    min_diff: crate::consensus::pow::Difficulty,
    last_diff: crate::consensus::pow::Difficulty,
) -> crate::consensus::pow::Difficulty {
    let mut timestamps = timestamps.to_vec();
    timestamps.sort_unstable();
    let final_size = difficulty_window - 2 * difficulty_cut;
    if timestamps.len() as u64 > final_size {
        let begin = (timestamps.len() as u64 - final_size + 1) / 2;
        let end = begin + final_size;
        timestamps = timestamps[begin as usize..end as usize].to_vec();
    }
    if timestamps.len() < 2 {
        return min_diff;
    }
    let num_blocks = timestamps.len() - 1;
    let time_delta = timestamps[num_blocks] - timestamps[0];
    let avg_block_time = time_delta / num_blocks as u32;
    let diff_change = (block_time as f32 / avg_block_time as f32).clamp(0.5f32, 2f32);
    let new_diff = rust_randomx::Difficulty::new(last_diff.0).scale(diff_change);
    std::cmp::max(
        crate::consensus::pow::Difficulty(new_diff.to_u32()),
        min_diff,
    )
}
