use crate::consensus::header::BabeAllowSlot::{Primary, SecondaryPlain, SecondaryVRF};
use crate::consensus::slots::Slot;
use crate::consensus::Error;
use crate::consensus::Result;
use serde::{Deserialize, Deserializer};
use test::RunIgnored::No;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Authority {
    pub public_key: [u8; 32],
    pub weight: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "tt", content = "cc")]
pub enum PreDigest {
    Primary(PrimaryDigest),
    SecondaryPlain(SecondaryPlainDigest),
    SecondaryVRF(SecondaryVRFDigest),
}

impl PreDigest {
    pub fn auth_idx(&self) -> u32 {
        match self {
            PreDigest::Primary(p) => p.auth_idx,
            PreDigest::SecondaryPlain(p) => p.auth_idx,
            PreDigest::SecondaryVRF(v) => v.auth_idx,
        }
    }

    pub fn slot(&self) -> Slot {
        match self {
            PreDigest::Primary(p) => p.slot_num,
            PreDigest::SecondaryPlain(p) => p.slot_num,
            PreDigest::SecondaryVRF(v) => v.slot_num,
        }
    }

    pub fn added_weight(&self) -> u32 {
        match self {
            PreDigest::Primary(_) => 1,
            PreDigest::SecondaryPlain(_) | PreDigest::SecondaryVRF(_) => 0,
        }
    }

    pub fn vrf_output(&self) -> Option<&[u8]> {
        match self {
            PreDigest::Primary(p) => Some(&p.vrf_output),
            PreDigest::SecondaryPlain(_) => None,
            PreDigest::SecondaryVRF(p) => Some(&p.vrf_output),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimaryDigest {
    pub auth_idx: u32,
    pub slot_num: Slot,
    pub vrf_output: [u8; 32],
    pub vrf_proof: [u8; 64],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecondaryPlainDigest {
    pub auth_idx: u32,
    pub slot_num: Slot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecondaryVRFDigest {
    pub auth_idx: u32,
    pub slot_num: Slot,
    pub vrf_output: [u8; 32],
    pub vrf_proof: [u8; 64],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BabeAllowSlot {
    Primary,
    SecondaryPlain,
    SecondaryVRF,
}

impl BabeAllowSlot {
    pub fn is_secondary_plain_slots(&self) -> bool {
        *self == Self::SecondaryPlain
    }
    pub fn is_secondary_vrf_slots(&self) -> bool {
        *self == Self::SecondaryVRF
    }
}

impl From<u8> for BabeAllowSlot {
    fn from(u: u8) -> Self {
        match u {
            0 => Primary,
            1 => SecondaryPlain,
            2 => SecondaryVRF,
            _ => unreachable!(),
        }
    }
}

impl BabeAllowSlot {
    pub fn from_slice(slice: &[u8]) -> Result<Self> {
        Ok(match slice.get(0) {
            Some(0) => Primary,
            Some(1) => SecondaryPlain,
            Some(2) => SecondaryVRF,
            _ => return Err(Error::BadBabeSlotType),
        })
    }
}
