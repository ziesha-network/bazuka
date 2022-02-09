use std::fmt::Formatter;
use std::time::{Duration, Instant, SystemTime};

use futures_timer::Delay;

use crate::consensus::{Error, Result};
use crate::core::header::Header;

pub enum SlotLenienceType {
    Linear,
    Exponential,
}

fn duration_now() -> Duration {
    let now = SystemTime::now();
    now.duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_else(|e| {
            panic!(
                "Current time {:?} is before unix epoch. Something is wrong: {:?}",
                now, e
            )
        })
}

fn duration_until_next_slot(slot_duration: Duration) -> Duration {
    let now = duration_now().as_millis();
    let next = (now + slot_duration.as_millis()) / slot_duration.as_millis();
    let gap = next * slot_duration.as_millis() - now;
    Duration::from_millis(gap as u64)
}

pub struct SlotInfo {
    pub slot: Slot,
    pub timestamp: u64,
    pub end_at: Instant,
    pub duration: Duration,
    pub chain_head: Header,
}

pub(crate) struct SlotStream {
    last_slot: Slot,
    slot_duration: Duration,
    delay: Option<Delay>,
}

impl SlotStream {
    pub fn new(slot_duration: Duration) -> Self {
        SlotStream {
            last_slot: Default::default(),
            slot_duration,
            delay: None,
        }
    }
}

impl SlotStream {
    pub async fn next_slot(&mut self) -> Result<SlotInfo> {
        loop {
            self.delay = match self.delay.take() {
                None => Some(Delay::new(duration_until_next_slot(self.slot_duration))),
                Some(d) => Some(d),
            };
            if let Some(d) = self.delay.take() {
                d.await;
            }
            let to_end = duration_until_next_slot(self.slot_duration);
            self.delay = Some(Delay::new(to_end));
            let end_at = Instant::now() + to_end;
            // @TODO: select chain head
            let chain_head: Header = Default::default();
            // @TODO: get current timestamp from backend
            let timestamp = 0u64;
            // @TODO: get current slot from backend
            // // 3 todo above must get data from backend because the `tree` in BABE is a `fork tree` actually
            // which it maybe update or prune later
            let slot = Slot::default();
            if slot > self.last_slot {
                self.last_slot = slot;
                break Ok(SlotInfo {
                    slot,
                    timestamp,
                    end_at,
                    duration: self.slot_duration,
                    chain_head,
                });
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, Default, Ord, Serialize, Deserialize)]
pub struct Slot(u64);

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
        write!(f, "{}", self.0)
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
