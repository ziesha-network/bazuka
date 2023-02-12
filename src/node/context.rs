use super::{
    Firewall, NodeError, NodeOptions, OutgoingSender, Peer, PeerAddress, PeerManager, Timestamp,
};
use crate::blockchain::{BlockAndPatch, Blockchain, BlockchainError, Mempool};
use crate::client::messages::SocialProfiles;
use crate::core::{ChainSourcedTx, Header, TransactionAndDelta};
use crate::utils;
use crate::wallet::TxBuilder;
use std::collections::HashMap;
use std::sync::Arc;

pub struct NodeContext<B: Blockchain> {
    pub firewall: Option<Firewall>,
    pub social_profiles: SocialProfiles,
    pub opts: NodeOptions,
    pub network: String,
    pub address: Option<PeerAddress>, // None means node is not exposed on the Internet
    pub shutdown: bool,
    pub outgoing: Arc<OutgoingSender>,
    pub blockchain: B,
    pub wallet: TxBuilder,
    pub peer_manager: PeerManager,
    pub timestamp_offset: i32,

    pub mempool: Mempool,

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
        let outdated_states = self.blockchain.get_outdated_contracts()?.len();
        Ok(self.address.map(|address| Peer {
            address,
            height,
            power,
            pub_key: self.wallet.get_address(),
            outdated_states,
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

        self.mempool.refresh(
            &self.blockchain,
            local_ts,
            self.opts.tx_max_time_alive,
            self.opts.tx_max_time_alive,
        )?;
        Ok(())
    }

    /// Is called whenever chain is extended or rolled back
    pub fn on_update(&mut self) -> Result<(), BlockchainError> {
        self.outdated_since = None;
        Ok(())
    }

    pub fn try_produce(
        &mut self,
        wallet: TxBuilder,
    ) -> Result<Option<BlockAndPatch>, BlockchainError> {
        let ts = self.network_timestamp();
        let raw_txs: Vec<TransactionAndDelta> = self
            .mempool
            .chain_sourced()
            .filter_map(|(tx, _)| {
                if let ChainSourcedTx::TransactionAndDelta(tx) = tx {
                    Some(tx.clone())
                } else {
                    None
                }
            })
            .collect();
        match self.blockchain.draft_block(ts, &raw_txs, &wallet, true) {
            Ok(draft) => {
                if let Some(draft) = draft {
                    self.blockchain
                        .extend(draft.block.header.number, &[draft.block.clone()])?;
                    self.on_update()?;
                    self.blockchain.update_states(&draft.patch.clone())?;
                    Ok(Some(draft))
                } else {
                    Ok(None)
                }
            }
            Err(e) => {
                log::warn!("Cannot draft a block! Error: {}", e);
                Ok(None)
            }
        }
    }
}
