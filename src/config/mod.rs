pub mod blockchain;

#[cfg(feature = "node")]
pub mod node;

pub const CODE: &str = "ZSH";
pub const SYMBOL: &str = "ℤ";
pub const UNIT_ZEROS: u8 = 9;
pub const UNIT: u64 = 10u64.pow(UNIT_ZEROS as u32);
