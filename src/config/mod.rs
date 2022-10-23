pub mod blockchain;
mod chaos;

#[cfg(feature = "node")]
pub mod node;

pub const CODE: &str = "ZIK";
pub const SYMBOL: &str = "ℤ";
pub const UNIT_ZEROS: u32 = 9;
pub const UNIT: u64 = 10u64.pow(UNIT_ZEROS);
