pub mod genesis;

#[cfg(feature = "node")]
pub mod punish;

pub const SYMBOL: &str = "ZIK";
pub const TOTAL_SUPPLY: u64 = 2_000_000_000_000_000_000_u64; // 2 Billion ZIK
pub const REWARD_RATIO: u64 = 100_000; // 1/100_000 -> 0.01% of Treasury Supply per block

// Delta means: block size + state size changes
pub const MAX_DELTA_SIZE: usize = 1024 * 1024 * 1024; // Bytes

// Every n blocks, recalculate difficulty
pub const BLOCK_TIME: usize = 60; // Seconds
pub const DIFFICULTY_CALC_INTERVAL: u64 = 128; // Blocks

pub const MAX_BLOCK_FETCH: u64 = 16; // Blocks

// 0 63 -> BAZUKA BASE KEY
// 64 2111 -> hash(blk#0)
// 2112 4159 -> hash(blk#2048)
// 4160 6207 -> hash(blk#4096)
// ...
pub const POW_BASE_KEY: &[u8] = b"BAZUKA BASE KEY";
pub const POW_KEY_CHANGE_DELAY: u64 = 64; // Blocks
pub const POW_KEY_CHANGE_INTERVAL: u64 = 2048; // Blocks

// New block's timestamp should be higher than median
// timestamp of 10 previous blocks
pub const MEDIAN_TIMESTAMP_COUNT: u64 = 10;

// Our Zero-Knowledge RAM will have 2^32 memory cells
pub const LOG_ZK_RAM_SIZE: usize = 32;

// Number of ZkStateDeltas we want to keep in our ZkStates
pub const NUM_STATE_DELTAS_KEEP: usize = 5;
