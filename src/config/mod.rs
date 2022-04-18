pub mod genesis;

#[cfg(feature = "node")]
pub mod punish;

pub const SYMBOL: &str = "ZIK";
pub const TOTAL_SUPPLY: u64 = 10000000000_000000000u64; // 10 Billion ZIK

// Delta means: block size + state size changes
pub const MAX_DELTA_SIZE: u32 = 1024 * 1024 * 1024; // Bytes

// Every n blocks, recalculate difficulty
pub const BLOCK_TIME: u32 = 60; // Seconds
pub const DIFFICULTY_CALC_INTERVAL: u32 = 128; // Blocks

// 0 63 -> BAZUKA BASE KEY
// 64 2111 -> hash(blk#0)
// 2112 4159 -> hash(blk#2048)
// 4160 6207 -> hash(blk#4096)
// ...
pub const POW_BASE_KEY: &[u8] = b"BAZUKA BASE KEY";
pub const POW_KEY_CHANGE_DELAY: usize = 64; // Blocks
pub const POW_KEY_CHANGE_INTERVAL: usize = 2048; // Blocks
