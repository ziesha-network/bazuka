use super::{
    Firewall, NodeError, NodeOptions, OutgoingSender, Peer, PeerAddress, PeerManager, Timestamp,
};
use crate::blockchain::{BlockAndPatch, Blockchain, BlockchainError, TransactionStats};
use crate::client::messages::SocialProfiles;
use crate::core::{ContractPayment, Header, Signer, TransactionAndDelta};
use crate::crypto::SignatureScheme;
use crate::utils;
use crate::wallet::Wallet;
use crate::zk;
use std::collections::HashMap;
use std::sync::Arc;

use crate::client::messages::Puzzle;

pub type BlockPuzzle = (BlockAndPatch, Puzzle);

pub struct NodeContext<B: Blockchain> {
    pub firewall: Option<Firewall>,
    pub social_profiles: SocialProfiles,
    pub opts: NodeOptions,
    pub network: String,
    pub pub_key: <Signer as SignatureScheme>::Pub,
    pub address: Option<PeerAddress>, // None means node is not exposed on the Internet
    pub shutdown: bool,
    pub outgoing: Arc<OutgoingSender>,
    pub blockchain: B,
    pub wallet: Option<Wallet>,
    pub peer_manager: PeerManager,
    pub timestamp_offset: i32,
    pub miner_puzzle: Option<BlockPuzzle>,

    pub mempool: HashMap<TransactionAndDelta, TransactionStats>,
    pub zero_mempool: HashMap<zk::MpnTransaction, TransactionStats>,
    pub contract_payment_mempool: HashMap<ContractPayment, TransactionStats>,

    pub outdated_since: Option<Timestamp>,
    pub banned_headers: HashMap<Header, Timestamp>,
}

impl<B: Blockchain> NodeContext<B> {
    pub fn local_timestamp(&self) -> u32 {
        utils::local_timestamp()
    }
    pub fn network_timestamp(&self) -> u32 {
        (self.local_timestamp() as i32 + self.timestamp_offset) as u32
    }
    pub fn punish_bad_behavior(&mut self, bad_peer: PeerAddress, secs: u32, reason: &str) {
        log::warn!("Peer {} is behaving bad! Reason: {}", bad_peer, reason);
        log::warn!("Punishing {} for {} seconds...", bad_peer, secs);
        self.peer_manager
            .punish_ip_for(self.local_timestamp(), bad_peer.ip(), secs);
    }
    pub fn punish_unresponsive(&mut self, bad_peer: PeerAddress) {
        log::warn!("Peer {} is unresponsive!", bad_peer);
        log::warn!("Moving peer {} to the candidate list!", bad_peer);
        self.peer_manager
            .mark_as_candidate(self.local_timestamp(), &bad_peer);
    }
    pub fn get_info(&self) -> Result<Option<Peer>, NodeError> {
        let height = self.blockchain.get_height()?;
        let power = self.blockchain.get_power()?;
        Ok(self.address.map(|address| Peer {
            address,
            height,
            power,
            pub_key: self.pub_key.clone(),
        }))
    }

    pub fn refresh(&mut self) -> Result<(), BlockchainError> {
        let local_ts = self.local_timestamp();
        self.peer_manager.refresh(local_ts);

        for (h, banned_at) in self.banned_headers.clone().into_iter() {
            if local_ts - banned_at > self.opts.state_unavailable_ban_time {
                self.banned_headers.remove(&h);
            }
        }

        if let Some(firewall) = &mut self.firewall {
            firewall.refresh(local_ts);
        }

        self.blockchain
            .cleanup_contract_payment_mempool(&mut self.contract_payment_mempool)?;
        self.blockchain.cleanup_mempool(&mut self.mempool)?;
        self.blockchain
            .cleanup_zero_mempool(&mut self.zero_mempool)?;

        if let Some(max) = self.opts.tx_max_time_alive {
            for (tx, stats) in self.mempool.clone().into_iter() {
                if local_ts - stats.first_seen > max {
                    self.mempool.remove(&tx);
                }
            }
            for (tx, stats) in self.contract_payment_mempool.clone().into_iter() {
                if local_ts - stats.first_seen > max {
                    self.contract_payment_mempool.remove(&tx);
                }
            }
            for (tx, stats) in self.zero_mempool.clone().into_iter() {
                if local_ts - stats.first_seen > max {
                    self.zero_mempool.remove(&tx);
                }
            }
        }
        Ok(())
    }

    /// Is called whenever chain is extended or rolled back
    pub fn on_update(&mut self) -> Result<(), BlockchainError> {
        self.outdated_since = None;
        self.miner_puzzle = None;
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
