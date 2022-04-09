use std::fmt::Formatter;
use std::time::{Duration, Instant, SystemTime};

use futures_timer::Delay;
use num_traits::Zero;

use super::{ChainSelector, CreateSlotAuxProvider, SlotAuxData};
use crate::core::Header;

/// type wrapper to express the proportion of a duration between two slot.
pub struct SlotProportion(f32);

impl SlotProportion {
    pub fn new(inner: f32) -> Self {
        Self(inner.clamp(0.0, 1.0))
    }

    pub fn get(&self) -> f32 {
        self.0
    }
}

pub fn proposing_remaining_duration(
    parent_slot: Option<Slot>,
    slot_info: &SlotInfo,
    block_proposal_slot_portion: &SlotProportion,
    max_block_proposal_slot_portion: Option<&SlotProportion>,
    slot_lenience_type: SlotLenienceType,
) -> Duration {
    let proposing_duration = slot_info
        .duration
        .mul_f32(block_proposal_slot_portion.get());

    let slot_remaining = slot_info
        .end_at
        .checked_duration_since(std::time::Instant::now())
        .unwrap_or_default();

    let proposing_duration = std::cmp::min(slot_remaining, proposing_duration);

    // genesis block
    if slot_info.chain_head.number.is_zero() {
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
            if let Some(ref max_block_proposal_slot_portion) = max_block_proposal_slot_portion {
                std::cmp::min(
                    lenient_proposing_duration,
                    slot_info
                        .duration
                        .mul_f32(max_block_proposal_slot_portion.get()),
                )
            } else {
                lenient_proposing_duration
            };

        log::debug!(
            target: "slots",
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

pub struct SlotInfo {
    pub slot: Slot,
    pub timestamp: u64,
    pub end_at: Instant,
    pub duration: Duration,
    pub chain_head: Header,
    /// @todo: proof maybe not exists as a vector
    pub parent_block_proof: Vec<u8>,
}

pub struct Ticker<ADP, CS> {
    last_slot: Slot,
    slot_duration: Duration,
    delay: Option<Delay>,
    aux_data_provider: ADP,
    chain_selector: CS,
}

impl<ADP, CS> Ticker<ADP, CS>
where
    ADP: CreateSlotAuxProvider,
    CS: ChainSelector,
{
    pub(super) fn new(slot_duration: Duration, aux_data_provider: ADP, chain_selector: CS) -> Self {
        Ticker {
            last_slot: 0.into(),
            slot_duration,
            delay: None,
            aux_data_provider,
            chain_selector,
        }
    }

    pub async fn next_slot(&mut self) -> super::Result<SlotInfo> {
        loop {
            self.delay = match self.delay.take() {
                None => Some(Delay::new(until_next_slot(self.slot_duration))),
                Some(d) => Some(d),
            };
            if let Some(d) = self.delay.take() {
                d.await;
            }
            let until_next_slot = until_next_slot(self.slot_duration);
            self.delay = Some(Delay::new(until_next_slot));
            let end_at = Instant::now() + until_next_slot;

            let chain_head = match self.chain_selector.best_chain().await {
                Ok(h) => h,
                Err(err) => {
                    log::warn!(target: "slots", "failed to tick next slot, no best header was found, {:?}", err);
                    self.delay.take();
                    continue;
                }
            };

            let aux_data = self
                .aux_data_provider
                .create_aux_provider(&chain_head.hash())
                .await?;

            if Instant::now() > end_at {
                log::warn!(
                    target: "slots",
                    "creating aux data took more time than the time left for current slot",
                );
            }

            let slot = aux_data.slot();
            let timestamp = aux_data.timestamp();
            let parent_block_proof = aux_data.parent_block_proof();

            if slot > self.last_slot {
                self.last_slot = slot;

                break Ok(SlotInfo {
                    slot,
                    timestamp,
                    end_at,
                    duration: self.slot_duration,
                    chain_head,
                    parent_block_proof,
                });
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, Default, Ord, serde::Serialize, serde::Deserialize)]
pub struct Slot(pub u64);

impl<T: Into<u64> + Copy> PartialEq<T> for Slot {
    fn eq(&self, other: &T) -> bool {
        self.0 == (*other).into()
    }
}

impl<T: Into<u64> + Copy> core::cmp::PartialOrd<T> for Slot {
    fn partial_cmp(&self, other: &T) -> Option<core::cmp::Ordering> {
        self.0.partial_cmp(&(*other).into())
    }
}

impl core::ops::Add for Slot {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl core::ops::Add<u64> for Slot {
    type Output = Self;

    fn add(self, rhs: u64) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl core::ops::Deref for Slot {
    type Target = u64;

    fn deref(&self) -> &u64 {
        &self.0
    }
}

impl std::fmt::Display for Slot {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl From<u64> for Slot {
    fn from(n: u64) -> Self {
        Self(n)
    }
}

impl From<Slot> for u64 {
    fn from(s: Slot) -> Self {
        s.0
    }
}

impl Slot {
    pub fn saturating_add<T: Into<u64>>(self, rhs: T) -> Self {
        Self(self.0.saturating_add(rhs.into()))
    }

    pub fn saturating_sub<T: Into<u64>>(self, rhs: T) -> Self {
        Self(self.0.saturating_sub(rhs.into()))
    }
}

fn until_now() -> Duration {
    let now = SystemTime::now();
    now.duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_else(|e| {
            panic!(
                "Current time {:?} is before unix epoch. Something is wrong: {:?}",
                now, e
            )
        })
}

fn until_next_slot(slot_duration: Duration) -> Duration {
    let now = until_now().as_millis();
    let next = (now + slot_duration.as_millis()) / slot_duration.as_millis();
    let gap = next * slot_duration.as_millis() - now;
    Duration::from_millis(gap as u64)
}

pub enum SlotLenienceType {
    Linear,
    Exponential,
}

impl SlotLenienceType {
    #[inline]
    pub fn as_str<'a>(&self) -> &'a str {
        match self {
            SlotLenienceType::Linear => "Linear",
            SlotLenienceType::Exponential => "Exponential",
        }
    }
}
