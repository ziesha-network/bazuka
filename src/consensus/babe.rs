use std::num::NonZeroU64;
use std::time::Duration;

use futures_timer::Delay;
use num_traits::Zero;

use crate::consensus::digest::{PreDigest, SecondaryPlainPreDigest};
use crate::consensus::slots::{
    proposing_remaining_duration, SlotLenienceType, SlotProportion, Ticker,
};
use crate::consensus::{ChainSelector, CreateSlotAuxProvider};
use crate::consensus::{Error, Result};
use crate::core::digest::*;
use crate::core::Header;

pub async fn start_babe_worker<ADP, CS>(
    slot_duration: Duration,
    aux_data_provider: ADP,
    chain_selector: CS,
    block_proposal_slot_portion: &SlotProportion,
    max_block_proposal_slot_portion: Option<&SlotProportion>,
) where
    ADP: CreateSlotAuxProvider,
    CS: ChainSelector,
{
    let mut ticker = Ticker::new(slot_duration, aux_data_provider, chain_selector);

    loop {
        let slot_info = match ticker.next_slot().await {
            Ok(info) => info,
            Err(err) => {
                log::error!(target: "slots", "failed to poll next slot {:?}", err);
                return;
            }
        };
        let parent_slot = find_pre_digest(&slot_info.chain_head)
            .ok()
            .map(|head| head.slot());
        let remain = proposing_remaining_duration(
            parent_slot,
            &slot_info,
            block_proposal_slot_portion,
            max_block_proposal_slot_portion,
            SlotLenienceType::Exponential,
        );

        let proposing_remaining = if remain == Duration::default() {
            log::debug!(
                target: "slots",
                "Skipping proposal slot {} since there's no time left to propose", &slot_info.slot,
            );
            continue;
        } else {
            Delay::new(remain)
        };
    }
}

pub fn find_pre_digest(header: &Header) -> Result<PreDigest> {
    if header.number.is_zero() {
        return Ok(PreDigest::SecondaryPlain(SecondaryPlainPreDigest {
            authority_index: 0,
            slot: Default::default(),
        }));
    }
    let mut pre_digest: Option<_> = None;
    for log in header.logs() {
        match (log, pre_digest.is_some()) {
            (Digest::PreDigest(_), true) => return Err(Error::MultiplePreDigests),
            (Digest::PreDigest(pre), false) => pre_digest = Some(pre.clone()),
            _ => {}
        }
    }
    pre_digest.ok_or_else(|| Error::NoPreDigest)
}

#[derive(Debug, Clone)]
pub struct Authority<T: AsRef<[u8]>> {
    pub weight: NonZeroU64,
    pub public_key: T,
}
