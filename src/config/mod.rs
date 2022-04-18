pub mod genesis;

#[cfg(feature = "node")]
pub mod punish;

pub const SYMBOL: &str = "ZIK";
pub const TOTAL_SUPPLY: u64 = 10000000000_000000000u64; // 10 Billion ZIK
pub const BLOCK_TIME: u32 = 60; // Seconds
pub const MAX_DELTA_SIZE: u32 = 1024 * 1024 * 1024; // Bytes

pub const POW_BASE_KEY: &[u8] = b"BAZUKA BASE KEY";
pub const POW_KEY_CHANGE_DELAY: usize = 64; // Blocks
pub const POW_KEY_CHANGE_INTERVAL: usize = 2048; // Blocks
