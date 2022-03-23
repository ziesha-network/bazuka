use super::{PeerAddress, PeerInfo, PeerStats};
use crate::blockchain::{Blockchain, BlockchainError};
use crate::core::Transaction;
use crate::utils;
use crate::wallet::Wallet;
use rand::seq::IteratorRandom;
use rand::RngCore;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TransactionStats {
    pub first_seen: u64,
}

pub struct NodeContext<B: Blockchain> {
    pub blockchain: B,
    pub wallet: Option<Wallet>,
    pub mempool: HashMap<Transaction, TransactionStats>,
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
    pub fn random_peers<R: RngCore>(
        &mut self,
        rng: &mut R,
        count: usize,
    ) -> HashMap<PeerAddress, PeerStats> {
        self.active_peers()
            .clone()
            .into_iter()
            .choose_multiple(rng, count)
            .into_iter()
            .collect()
    }
    pub fn active_peers(&mut self) -> HashMap<PeerAddress, PeerStats> {
        self.peers
            .iter_mut()
            .filter_map(|(k, v)| {
                if !v.is_punished() {
                    Some((k.clone(), v.clone()))
                } else {
                    None
                }
            })
            .collect()
    }
}
