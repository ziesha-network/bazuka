use super::{PeerAddress, PeerInfo, PeerStats};
use crate::blockchain::{Blockchain, BlockchainError};
use crate::utils;
use std::collections::HashMap;

pub struct NodeContext<B: Blockchain> {
    pub blockchain: B,
    pub peers: HashMap<PeerAddress, PeerStats>,
    pub timestamp_offset: i64,
}

impl<B: Blockchain> NodeContext<B> {
    pub fn timestamp(&self) -> u64 {
        (utils::timestamp() as i64 + self.timestamp_offset) as u64
    }
    pub fn get_info(&self) -> Result<PeerInfo, BlockchainError> {
        Ok(PeerInfo {
            height: self.blockchain.get_height()?,
        })
    }
}
