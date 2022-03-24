use crate::consensus::babe;
use crate::consensus::babe::Authority;
use crate::consensus::slots::Slot;
use crate::crypto::{EdDSAPublicKey, PublicKey};

const RANDOMNESS_LEN: usize = 32;

/// Epoch Information
pub struct Epoch<P: PublicKey> {
    pub index: u64,

    pub start_slot_number: Slot,

    pub duration: u64,

    pub authorities: Vec<Authority<P>>,

    pub randomness: [u8; RANDOMNESS_LEN],

    pub c: (u64, u64),

    pub allow_slots: AllowSlot,
}

impl<P: PublicKey> Epoch<P> {
    pub fn increment(&self, desc: NextEpochDescriptor<P>, config: EpochConfiguration) -> Self {
        Epoch {
            index: self.index + 1,
            start_slot_number: self.start_slot_number + self.duration,
            duration: self.duration,
            authorities: desc.authorities,
            randomness: desc.randomness,
            c: config.c,
            allow_slots: config.allow_slots,
        }
    }

    pub fn start_slot(&self) -> Slot {
        self.start_slot_number
    }

    pub fn end_slot(&self) -> Slot {
        self.start_slot_number + self.duration
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllowSlot {
    Primary,
    PrimaryAndSecondaryPlain,
    PrimaryAndSecondaryVFR,
}

#[derive(Debug, Clone)]
pub struct NextEpochDescriptor<P: PublicKey> {
    pub authorities: Vec<Authority<P>>,
    pub randomness: [u8; RANDOMNESS_LEN],
}

#[derive(Clone)]
pub struct EpochConfiguration {
    pub c: (u64, u64),
    pub allow_slots: AllowSlot,
}
