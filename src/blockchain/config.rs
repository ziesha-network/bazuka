use super::BlockAndPatch;
use crate::core::{Address, TokenId};
use crate::mpn::MpnConfig;
use std::collections::HashSet;

#[derive(Clone)]
pub struct BlockchainConfig {
    pub limited_miners: Option<HashSet<Address>>,
    pub genesis: BlockAndPatch,
    pub reward_ratio: u64,
    pub max_block_size: usize,
    pub max_delta_count: usize,
    pub ziesha_token_id: TokenId,
    pub mpn_config: MpnConfig,
    pub testnet_height_limit: Option<u64>,
    pub max_memo_length: usize,
    pub slot_duration: u32,
    pub slot_per_epoch: u32,
    pub chain_start_timestamp: u32,
    pub check_validator: bool,
    pub max_validator_commision: u8,
}
