use super::{NodeOptions, OutgoingSender, Peer, PeerAddress, PeerInfo, Timestamp};
use crate::blockchain::{BlockAndPatch, Blockchain, BlockchainError, TransactionStats};
use crate::core::{ContractPayment, Header, Signer, TransactionAndDelta};
use crate::crypto::SignatureScheme;
use crate::utils;
use crate::wallet::Wallet;
use crate::zk;
use rand::seq::IteratorRandom;
use rand::RngCore;
use std::collections::HashMap;
use std::sync::Arc;

use crate::client::messages::Puzzle;

pub type BlockPuzzle = (BlockAndPatch, Puzzle);

pub struct NodeContext<B: Blockchain> {
    pub opts: NodeOptions,
    pub pub_key: <Signer as SignatureScheme>::Pub,
    pub address: PeerAddress,
    pub shutdown: bool,
    pub outgoing: Arc<OutgoingSender>,
    pub blockchain: B,
    pub wallet: Option<Wallet>,
    pub peers: HashMap<PeerAddress, Peer>,
    pub timestamp_offset: i32,
    pub miner_puzzle: Option<BlockPuzzle>,

    pub mempool: HashMap<TransactionAndDelta, TransactionStats>,
    pub zero_mempool: HashMap<zk::ZeroTransaction, TransactionStats>,
    pub dw_mempool: HashMap<ContractPayment, TransactionStats>,

    pub outdated_since: Option<Timestamp>,
    pub banned_headers: HashMap<Header, Timestamp>,
}

impl<B: Blockchain> NodeContext<B> {
    pub fn network_timestamp(&self) -> u32 {
        (utils::local_timestamp() as i32 + self.timestamp_offset) as u32
    }
    pub fn punish(&mut self, bad_peer: PeerAddress, secs: u32) {
        self.peers
            .entry(bad_peer)
            .and_modify(|stats| stats.punish(secs, self.opts.max_punish));
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

    pub fn get_puzzle(&mut self, wallet: Wallet) -> Result<Option<BlockPuzzle>, BlockchainError> {
        let ts = self.network_timestamp();
        let draft = self
            .blockchain
            .draft_block(ts, &mut self.mempool, &wallet, true)?;
        if let Some(draft) = draft {
            let puzzle = Puzzle {
                key: hex::encode(self.blockchain.pow_key(draft.block.header.number)?),
                blob: hex::encode(bincode::serialize(&draft.block.header).unwrap()),
                offset: 80,
                size: 8,
                target: draft.block.header.proof_of_work.target,
            };
            Ok(Some((draft, puzzle)))
        } else {
            Ok(None)
        }
    }
}
