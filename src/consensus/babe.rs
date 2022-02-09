use std::hash::Hash;
use std::sync::Arc;
use std::time::Duration;

use futures_timer::Delay;
use log::{debug, error};
use num_traits::Num;

use crate::block::Header;
use crate::consensus::auth::claim_slot_use_keys;
use crate::consensus::epochs::EpochChanges;
use crate::consensus::forktree::ForkTree;
use crate::consensus::header::{Authority, BabeAllowSlot, PreDigest, SecondaryPlainDigest};
use crate::consensus::slots::SlotLenienceType::Exponential;
use crate::consensus::slots::{Slot, SlotInfo, SlotLenienceType, SlotStream};
use crate::consensus::{Error, Result};
use crate::core::header::{DigestItem, Header};
use crate::core::U256;
use crate::keystore::SyncCryptoApi;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenesisConf {
    pub slot_duration: u64,
    pub epoch_length: u64,
    pub c: (u64, u64),
    pub genesis_auths: Vec<(Authority, u64)>,
    pub randomness: [u8; 32],
    pub allow_slots: BabeAllowSlot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Epoch {
    pub authorities: Vec<(Authority, u64)>,
    pub randomness: [u8; 32],
    pub epoch_idx: u64,
    pub start_slot: Slot,
    pub duration: u64,
    pub c: (u64, u64),
    pub allow_slots: BabeAllowSlot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextEpochAuths {
    pub authorities: Vec<(Authority, u64)>,
    pub randomness: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpochConf {
    pub c: (u64, u64),
    pub allow_slots: BabeAllowSlot,
}

impl Epoch {
    pub fn increment(&self, next_auths: NextEpochAuths, conf: EpochConf) -> Self {
        Epoch {
            epoch_idx: self.epoch_idx + 1,
            start_slot: self.start_slot + self.duration,
            duration: self.duration,
            authorities: next_auths.authorities,
            randomness: next_auths.randomness,
            c: conf.c,
            allow_slots: conf.allow_slots,
        }
    }

    pub fn start_slot(&self) -> Slot {
        self.start_slot
    }
    pub fn end_slot(&self) -> Slot {
        self.start_slot + self.duration
    }

    pub fn genesis(genesis: &GenesisConf, slot: Slot) -> Self {
        Epoch {
            epoch_idx: 0,
            start_slot: slot,
            duration: genesis.epoch_length,
            authorities: genesis.genesis_auths.clone(),
            randomness: genesis.randomness,
            c: genesis.c,
            allow_slots: genesis.allow_slots,
        }
    }
}

pub async fn start_slot_worker(
    slot: u64,
    key_store: Arc<dyn SyncCryptoApi>,
    block_proposal_slot_portion: f32,
    max_block_proposal_slot_portion: Option<f32>,
) {
    let mut stream = SlotStream::new(std::time::Duration::from_millis(slot));
    let mut worker = SlotWorker {
        block_proposal_slot_portion,
        max_block_proposal_slot_portion,
        key_store,
        epoch_changes: Default::default(),
    };

    loop {
        // next slot
        let slot_info = match stream.next_slot().await {
            Ok(info) => info,
            Err(e) => {
                error!(target: "slots", "while polling next slot: {:?}", e);
                return;
            }
        };

        worker.on_slot(slot_info)
    }
}

// C: backend data provider
pub struct SlotWorker {
    block_proposal_slot_portion: f32,
    max_block_proposal_slot_portion: Option<f32>,
    key_store: Arc<dyn SyncCryptoApi>,
    // @TODO: define block and then there would be `Hash` and `Num`
    epoch_changes: EpochChanges<Hash, U256>,
}

impl SlotWorker {
    pub fn on_slot(&mut self, slot_info: SlotInfo) {
        let (timestamp, slot, header) = (slot_info.timestamp, slot_info.slot, slot_info.chain_head);
        let parent_slot = find_pre_digest(&header).ok().map(|d| d.slot());
        let remain = calculate_remaining_duration(
            parent_slot,
            &slot_info,
            self.block_proposal_slot_portion,
            self.max_block_proposal_slot_portion,
            Exponential,
        );
        let proposing_remain = if remain == Default::default() {
            // skip this
            None
        } else {
            Delay::new(remain)
        };
        // @TODO: check sync state, if the node is running in block-syncing mode then return

        // @TODO: get epoch data from {backend + epoch-changes(fork tree)} by header in slot info
        // let epoch = self.epoch_changes.get_by@slot_info.best-chain-head and @slot
        let epoch = Epoch {
            authorities: Vec::new(),
            randomness: [0; 32],
            epoch_idx: 0,
            start_slot: Default::default(),
            duration: 0,
            c: (0, 0),
            allow_slots: BabeAllowSlot::Primary,
        };
        // @TODO: update epoch's authorities
        // genesis authorities or tmpdata in epoch-changes or backend

        // claim epoch
        let authorities = epoch
            .authorities
            .iter()
            .enumerate()
            .map(|(index, a)| (a.0.clone(), index))
            .collect::<Vec<_>>();
        let claim = claim_slot_use_keys(slot, &epoch, &self.key_store, &authorities);
        // wait for next slot
        if let Some(claim) = claim {
            // @TODO: proposing logs in async with Delay(proposing_remain)
            let logs = serde_json::to_vec(&claim.0).unwrap();
        }

        // @TODO: sync blocks from blockchain anyway
    }
}

pub fn find_pre_digest(header: &Header) -> Result<PreDigest> {
    if header.number == 0 {
        return Ok(PreDigest::SecondaryPlain(SecondaryPlainDigest {
            auth_idx: 0,
            slot_num: Default::default(),
        }));
    }
    let mut pre_digest: Option<_> = None;
    for log in header.digests.logs() {
        if pre_digest.is_none() {
            match log {
                DigestItem::PreRuntime(p) => pre_digest = Some(DigestItem::PreRuntime(p.clone())),
                _ => continue,
            }
        } else {
            return Err(Error::MultiplePreRuntimeDigests);
        }
    }
    pre_digest.map_or_else(Err(Error::NoPreRuntimeDigests), |d| {
        let v = match d {
            DigestItem::PreRuntime(v) => v[..],
            DigestItem::Consensus(v) => v[..],
            DigestItem::Seal(v) => v[..],
        };
        serde_json::from_slice(&v).map_or_else(|e| Err(Error::BadDigestFormat(e)), |d| Ok(d))
    })
}

pub fn calculate_remaining_duration(
    parent_slot: Option<Slot>,
    slot_info: &SlotInfo,
    block_proposing_portion: f32,
    max_block_proposing_portion: Option<f32>,
    slot_lenience_type: SlotLenienceType,
) -> Duration {
    let proposing_duration = slot_info.duration.mul_f32(block_proposing_portion);
    let slot_remaining = slot_info
        .end_at
        .checked_duration_since(std::time::Instant::now())
        .unwrap_or_default();
    let proposing_duration = std::cmp::min(slot_remaining, proposing_duration);
    if slot_info.chain_head.number == 0 {
        // genesis block
        return proposing_duration;
    }
    let parent_slot = match parent_slot {
        Some(parent_slot) => parent_slot,
        None => return proposing_duration,
    };
    let slot_lenience = match slot_lenience_type {
        SlotLenienceType::Exponential => slot_lenience_exponential(parent_slot, slot_info),
        SlotLenienceType::Linear => slot_lenience_linear(parent_slot, slot_info),
    };

    if let Some(slot_lenience) = slot_lenience {
        let lenient_proposing_duration =
            proposing_duration + slot_lenience.mul_f32(block_proposal_slot_portion.get());

        // if we defined a maximum portion of the slot for proposal then we must make sure the
        // lenience doesn't go over it
        let lenient_proposing_duration =
            if let Some(max_block_proposing_portion) = max_block_proposing_portion {
                std::cmp::min(
                    lenient_proposing_duration,
                    slot_info.duration.mul_f32(max_block_proposing_portion),
                )
            } else {
                lenient_proposing_duration
            };

        debug!(
            target: log_target,
            "No block for {} slots. Applying {} lenience, total proposing duration: {}",
            slot_info.slot.saturating_sub(parent_slot + 1),
            slot_lenience_type.as_str(),
            lenient_proposing_duration.as_secs(),
        );

        lenient_proposing_duration
    } else {
        proposing_duration
    }
}

pub fn slot_lenience_linear(parent_slot: Slot, slot_info: &SlotInfo) -> Option<Duration> {
    // never give more than 20 times more lenience.
    const BACKOFF_CAP: u64 = 20;

    let skipped_slots = *slot_info.slot.saturating_sub(parent_slot + 1);

    if skipped_slots == 0 {
        None
    } else {
        let slot_lenience = std::cmp::min(skipped_slots, BACKOFF_CAP);
        // We cap `slot_lenience` to `20`, so it should always fit into an `u32`.
        Some(slot_info.duration * (slot_lenience as u32))
    }
}

pub fn slot_lenience_exponential(parent_slot: Slot, slot_info: &SlotInfo) -> Option<Duration> {
    // never give more than 2^this times the lenience.
    const BACKOFF_CAP: u64 = 7;

    // how many slots it takes before we double the lenience.
    const BACKOFF_STEP: u64 = 2;

    let skipped_slots = *slot_info.slot.saturating_sub(parent_slot + 1);

    if skipped_slots == 0 {
        None
    } else {
        let slot_lenience = skipped_slots / BACKOFF_STEP;
        let slot_lenience = std::cmp::min(slot_lenience, BACKOFF_CAP);
        let slot_lenience = 1 << slot_lenience;
        Some(slot_lenience * slot_info.duration)
    }
}
