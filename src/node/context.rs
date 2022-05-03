use super::{PeerAddress, PeerInfo, PeerStats};
use crate::blockchain::{Blockchain, BlockchainError};
use crate::core::Transaction;
use crate::utils;
use crate::wallet::Wallet;
use rand::seq::IteratorRandom;
use rand::RngCore;
use std::collections::HashMap;

#[cfg(feature = "pow")]
use {super::api::messages::Puzzle, crate::core::Block};

#[derive(Debug, Clone)]
pub struct TransactionStats {
    pub first_seen: u32,
}

#[cfg(feature = "pow")]
pub type BlockPuzzle = (Block, Puzzle);

#[cfg(feature = "pow")]
pub struct Miner {
    pub block_puzzle: Option<BlockPuzzle>,
    pub webhook: Option<String>,
}

pub struct NodeContext<B: Blockchain> {
    pub blockchain: B,
    pub wallet: Option<Wallet>,
    pub mempool: HashMap<Transaction, TransactionStats>,
    pub peers: HashMap<PeerAddress, PeerStats>,
    pub timestamp_offset: i32,
    #[cfg(feature = "pow")]
    pub miner: Option<Miner>,
}

impl<B: Blockchain> NodeContext<B> {
    pub fn network_timestamp(&self) -> u32 {
        (utils::local_timestamp() as i32 + self.timestamp_offset) as u32
    }
    pub fn punish(&mut self, bad_peer: PeerAddress, secs: u32) {
        self.peers
            .entry(bad_peer)
            .and_modify(|stats| stats.punish(secs));
    }
    pub fn get_info(&self) -> Result<PeerInfo, BlockchainError> {
        Ok(PeerInfo {
            height: self.blockchain.get_height()?,
            #[cfg(feature = "pow")]
            power: self.blockchain.get_power()?,
        })
    }
    pub fn random_peers<R: RngCore>(
        &self,
        rng: &mut R,
        count: usize,
    ) -> HashMap<PeerAddress, PeerStats> {
        self.active_peers()
            .into_iter()
            .choose_multiple(rng, count)
            .into_iter()
            .collect()
    }
    pub fn active_peers(&self) -> HashMap<PeerAddress, PeerStats> {
        self.peers
            .iter()
            .filter_map(|(k, v)| {
                if !v.is_punished() {
                    Some((*k, v.clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    #[cfg(feature = "pow")]
    pub fn get_puzzle(&self, wallet: Wallet) -> Result<BlockPuzzle, BlockchainError> {
        let txs = self.mempool.keys().cloned().collect::<Vec<_>>();
        let ts = self.network_timestamp();
        let block = self.blockchain.draft_block(ts, &txs, &wallet)?;
        let puzzle = Puzzle {
            key: hex::encode(self.blockchain.pow_key(block.header.number as usize)?),
            blob: hex::encode(bincode::serialize(&block.header).unwrap()),
            offset: 112,
            size: 8,
            target: block.header.proof_of_work.target,
        };
        Ok((block, puzzle))
    }
}
