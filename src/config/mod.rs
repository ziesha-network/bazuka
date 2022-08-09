pub mod blockchain;

#[cfg(feature = "node")]
pub mod node;

pub const CODE: &str = "ZIK";
pub const SYMBOL: &str = "ℤ";
pub const MAX_BLOCK_FETCH: u64 = 16; // Blocks
