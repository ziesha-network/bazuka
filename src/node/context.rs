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
    pub fn network_timestamp(&self) -> u64 {
        (utils::local_timestamp() as i64 + self.timestamp_offset) as u64
    }
    pub fn get_info(&self) -> Result<PeerInfo, BlockchainError> {
        Ok(PeerInfo {
            height: self.blockchain.get_height()?,
        })
    }
    pub fn active_peers(&mut self) -> Vec<PeerAddress> {
        self.peers
            .iter_mut()
            .filter_map(|(k, v)| {
                if v.info.is_some() && !v.is_punished() {
                    Some(k.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}
