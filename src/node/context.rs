use super::{OutgoingSender, Peer, PeerAddress, PeerInfo};
use crate::blockchain::{Blockchain, BlockchainError};
use crate::core::TransactionAndPatch;
use crate::utils;
use crate::wallet::Wallet;
use rand::seq::IteratorRandom;
use rand::RngCore;
use std::collections::HashMap;
use std::sync::Arc;

use {super::api::messages::Puzzle, crate::core::Block};

#[derive(Debug, Clone)]
pub struct TransactionStats {
    pub first_seen: u32,
}

pub type BlockPuzzle = (Block, Puzzle);

pub struct Miner {
    pub block_puzzle: Option<BlockPuzzle>,
    pub webhook: Option<String>,
}

pub struct NodeContext<B: Blockchain> {
    pub address: PeerAddress,
    pub shutdown: bool,
    pub outgoing: Arc<OutgoingSender>,
    pub blockchain: B,
    pub wallet: Option<Wallet>,
    pub mempool: HashMap<TransactionAndPatch, TransactionStats>,
    pub peers: HashMap<PeerAddress, Peer>,
    pub timestamp_offset: i32,

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

            power: self.blockchain.get_power()?,
        })
    }
    pub fn random_peers<R: RngCore>(&self, rng: &mut R, count: usize) -> Vec<Peer> {
        self.active_peers()
            .into_iter()
            .choose_multiple(rng, count)
            .into_iter()
            .collect()
    }
    pub fn active_peers(&self) -> Vec<Peer> {
        self.peers
            .values()
            .cloned()
            .filter(|p| !p.is_punished() && p.address != self.address)
            .collect()
    }

    pub fn get_puzzle(&self, wallet: Wallet) -> Result<BlockPuzzle, BlockchainError> {
        let txs = self.mempool.keys().cloned().collect::<Vec<_>>();
        let ts = self.network_timestamp();
        let block = self.blockchain.draft_block(ts, &txs, &wallet)?;
        let puzzle = Puzzle {
            key: hex::encode(self.blockchain.pow_key(block.header.number)?),
            blob: hex::encode(bincode::serialize(&block.header).unwrap()),
            offset: 80,
            size: 8,
            target: block.header.proof_of_work.target,
        };
        Ok((block, puzzle))
    }
}
