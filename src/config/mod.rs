pub mod genesis;

#[cfg(feature = "node")]
pub mod punish;

pub const SYMBOL: &str = "ZIK";
pub const MAX_BLOCK_FETCH: u64 = 16; // Blocks

// Number of ZkStateDeltas we want to keep in our ZkStates
pub const NUM_STATE_DELTAS_KEEP: usize = 5;
