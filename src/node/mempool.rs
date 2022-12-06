use crate::blockchain::TransactionStats;
use crate::core::{ChainSourcedTx, MpnSourcedTx};
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct Mempool {
    pub chain_sourced: HashMap<ChainSourcedTx, TransactionStats>,
    pub mpn_sourced: HashMap<MpnSourcedTx, TransactionStats>,
}
