use super::{Firewall, NodeOptions, OutgoingSender, Peer, PeerAddress, PeerInfo, Timestamp};
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
    pub firewall: Firewall,
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
    pub contract_payment_mempool: HashMap<ContractPayment, TransactionStats>,

    pub outdated_since: Option<Timestamp>,
    pub banned_headers: HashMap<Header, Timestamp>,
}

impl<B: Blockchain> NodeContext<B> {
    pub fn network_timestamp(&self) -> u32 {
        (utils::local_timestamp() as i32 + self.timestamp_offset) as u32
    }
    pub fn punish_bad_behavior(&mut self, bad_peer: PeerAddress, secs: u32, reason: &str) {
        log::warn!("Peer {} is behaving bad! Reason: {}", bad_peer, reason);
        log::warn!("Punishing {} for {} seconds...", bad_peer, secs);
        self.firewall.punish_bad(bad_peer.0.ip(), secs);
    }
    pub fn punish_unresponsive(&mut self, bad_peer: PeerAddress) {
        log::warn!("Peer {} is unresponsive!", bad_peer);
        log::warn!(
            "Punishing {} for {} seconds...",
            bad_peer,
            self.opts.no_response_punish
        );
        self.firewall.punish_unresponsive(
            bad_peer.0.ip(),
            self.opts.no_response_punish,
            self.opts.max_punish,
        );
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
            .filter(|p| self.firewall.outgoing_permitted(p.address) && p.address != self.address)
            .collect()
    }

    pub fn refresh(&mut self) -> Result<(), BlockchainError> {
        // TODO: Remove all inactive peers

        let ts = self.network_timestamp();
        for (h, banned_at) in self.banned_headers.clone().into_iter() {
            if ts - banned_at > self.opts.state_unavailable_ban_time {
                self.banned_headers.remove(&h);
            }
        }

        self.firewall.refresh();
        self.blockchain
            .cleanup_contract_payment_mempool(&mut self.contract_payment_mempool)?;
        self.blockchain.cleanup_mempool(&mut self.mempool)?;
        self.blockchain
            .cleanup_zero_mempool(&mut self.zero_mempool)?;

        if let Some(max) = self.opts.tx_max_time_alive {
            for (tx, stats) in self.mempool.clone().into_iter() {
                if ts - stats.first_seen > max {
                    self.mempool.remove(&tx);
                }
            }
            for (tx, stats) in self.contract_payment_mempool.clone().into_iter() {
                if ts - stats.first_seen > max {
                    self.contract_payment_mempool.remove(&tx);
                }
            }
            for (tx, stats) in self.zero_mempool.clone().into_iter() {
                if ts - stats.first_seen > max {
                    self.zero_mempool.remove(&tx);
                }
            }
        }
        Ok(())
    }

    pub fn get_puzzle(&mut self, wallet: Wallet) -> Result<Option<BlockPuzzle>, BlockchainError> {
        let ts = self.network_timestamp();
        let draft = self
            .blockchain
            .draft_block(ts, &self.mempool, &wallet, true)?;
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
