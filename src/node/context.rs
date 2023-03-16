use super::{
    Firewall, NodeError, NodeOptions, OutgoingSender, Peer, PeerAddress, PeerManager, Timestamp,
};
use crate::blockchain::{BlockAndPatch, Blockchain, BlockchainError, Mempool};
use crate::client::messages::{SocialProfiles, ValidatorClaim};
use crate::core::{ChainSourcedTx, Header, MpnAddress, MpnSourcedTx, TransactionAndDelta};
use crate::mpn::{MpnWorkPool, MpnWorker};
use crate::node::KvStore;
use crate::utils;
use crate::wallet::TxBuilder;
use std::collections::HashMap;
use std::sync::Arc;

pub struct NodeContext<K: KvStore, B: Blockchain<K>> {
    pub firewall: Option<Firewall>,
    pub social_profiles: SocialProfiles,
    pub opts: NodeOptions,
    pub network: String,
    pub address: Option<PeerAddress>, // None means node is not exposed on the Internet
    pub shutdown: bool,
    pub outgoing: Arc<OutgoingSender>,
    pub blockchain: B,
    pub validator_wallet: TxBuilder,
    pub user_wallet: TxBuilder,
    pub peer_manager: PeerManager,
    pub timestamp_offset: i32,
    pub validator_claim: Option<ValidatorClaim>,

    pub mpn_workers: HashMap<MpnAddress, MpnWorker>,
    pub mpn_work_pool: Option<MpnWorkPool>,

    pub mempool: Mempool,

    pub outdated_since: Option<Timestamp>,
    pub banned_headers: HashMap<Header, Timestamp>,
    pub _phantom: std::marker::PhantomData<K>,
}

impl<K: KvStore, B: Blockchain<K>> NodeContext<K, B> {
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
        let outdated_states = self.blockchain.get_outdated_contracts()?.len();
        Ok(self.address.map(|address| Peer {
            address,
            height,
            pub_key: self.validator_wallet.get_address(),
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

        Ok(())
    }

    pub fn mempool_add_chain_sourced(
        &mut self,
        is_local: bool,
        tx: ChainSourcedTx,
    ) -> Result<(), BlockchainError> {
        let local_ts = self.local_timestamp();
        self.mempool
            .add_chain_sourced(&self.blockchain, tx, is_local, local_ts)?;
        Ok(())
    }

    pub fn mempool_add_mpn_sourced(
        &mut self,
        is_local: bool,
        tx: MpnSourcedTx,
    ) -> Result<(), BlockchainError> {
        let local_ts = self.local_timestamp();
        self.mempool
            .add_mpn_sourced(&self.blockchain, tx, is_local, local_ts)?;
        Ok(())
    }

    /// Is called whenever chain is extended or rolled back
    pub fn on_update(&mut self) -> Result<(), BlockchainError> {
        self.outdated_since = None;
        let local_ts = self.local_timestamp();
        self.mempool.refresh(
            &self.blockchain,
            local_ts,
            self.opts.tx_max_time_alive,
            self.opts.tx_max_time_alive,
        )?;
        Ok(())
    }

    pub fn update_validator_claim(
        &mut self,
        claim: ValidatorClaim,
    ) -> Result<bool, BlockchainError> {
        if self.validator_claim != Some(claim.clone()) {
            // Only handle one winner!
            if let Some(curr_claim) = self.validator_claim.clone() {
                let (epoch_curr, slot_curr) = self.blockchain.epoch_slot(curr_claim.timestamp);
                let (epoch_req, slot_req) = self.blockchain.epoch_slot(claim.timestamp);
                if epoch_curr == epoch_req && slot_curr == slot_req {
                    return Ok(false);
                }
            }
            let ts = self.network_timestamp();
            if self
                .blockchain
                .is_validator(ts, claim.address.clone(), claim.proof.clone())?
                && claim.verify_signature()
            {
                self.validator_claim = Some(claim.clone());
                log::info!("Address {} is the validator!", claim.address);
                return Ok(true);
            }
        }
        Ok(false)
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
