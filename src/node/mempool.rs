use crate::blockchain::TransactionStats;
use crate::core::{MpnPayment, TransactionAndDelta};
use crate::zk::MpnTransaction;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct Mempool {
    pub tx: HashMap<TransactionAndDelta, TransactionStats>,
    pub zk: HashMap<MpnTransaction, TransactionStats>,
    pub tx_zk: HashMap<MpnPayment, TransactionStats>,
}
