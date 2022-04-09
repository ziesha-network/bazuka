use std::fmt::Debug;

use serde::ser::SerializeTuple;
use serde::{Deserializer, Serializer};

use super::slots::Slot;
use crate::utils;

const OUTPUT_LEN: usize = 32;
const PROOF_LEN: usize = 64;

/// A slot assignment pre-digest
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PreDigest {
    Primary(PrimaryPreDigest),
    SecondaryPlain(SecondaryPlainPreDigest),
    SecondaryVRF(SecondaryVRFPreDigest),
}

impl PreDigest {
    pub fn slot(&self) -> Slot {
        match self {
            PreDigest::Primary(primary) => primary.slot,
            PreDigest::SecondaryPlain(secondary) => secondary.slot,
            PreDigest::SecondaryVRF(secondary) => secondary.slot,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PrimaryPreDigest {
    pub authority_index: u32,
    pub slot: Slot,
    #[serde(serialize_with = "se_vrf_output", deserialize_with = "der_vrf_output")]
    pub vrf_output: [u8; OUTPUT_LEN],
    #[serde(serialize_with = "se_vrf_proof", deserialize_with = "der_vrf_proof")]
    pub vrf_proof: [u8; PROOF_LEN],
}

impl PrimaryPreDigest {
    pub fn new(
        authority_index: u32,
        slot: Slot,
        vrf_output: [u8; OUTPUT_LEN],
        vrf_proof: [u8; PROOF_LEN],
    ) -> Self {
        Self {
            authority_index,
            slot,
            vrf_output,
            vrf_proof,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SecondaryPlainPreDigest {
    pub authority_index: u32,
    /// Slot
    pub slot: Slot,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SecondaryVRFPreDigest {
    pub authority_index: u32,
    pub slot: Slot,
    #[serde(serialize_with = "se_vrf_output", deserialize_with = "der_vrf_output")]
    pub vrf_output: [u8; OUTPUT_LEN],
    #[serde(serialize_with = "se_vrf_proof", deserialize_with = "der_vrf_proof")]
    pub vrf_proof: [u8; PROOF_LEN],
}

impl SecondaryVRFPreDigest {
    pub fn new(
        authority_index: u32,
        slot: Slot,
        vrf_output: [u8; OUTPUT_LEN],
        vrf_proof: [u8; PROOF_LEN],
    ) -> Self {
        Self {
            authority_index,
            slot,
            vrf_output,
            vrf_proof,
        }
    }
}

fn se_vrf_output<S>(v: &[u8; OUTPUT_LEN], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut t = serializer.serialize_tuple(OUTPUT_LEN)?;
    for b in v.into_iter() {
        t.serialize_element(b)?;
    }
    t.end()
}

fn der_vrf_output<'de, D>(deserializer: D) -> Result<[u8; OUTPUT_LEN], D::Error>
where
    D: Deserializer<'de>,
{
    let output = deserializer.deserialize_tuple(OUTPUT_LEN, utils::ArrayVisitor::new())?;
    Ok(output)
}

fn se_vrf_proof<S>(v: &[u8; PROOF_LEN], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut t = serializer.serialize_tuple(PROOF_LEN)?;
    for b in v.into_iter() {
        t.serialize_element(b)?;
    }
    t.end()
}

fn der_vrf_proof<'de, D>(deserializer: D) -> Result<[u8; PROOF_LEN], D::Error>
where
    D: Deserializer<'de>,
{
    let output: [u8; PROOF_LEN] =
        deserializer.deserialize_tuple(PROOF_LEN, utils::ArrayVisitor::new())?;
    // let proof = VRFProof::from_bytes(&output)
    //     .map_err(|err| D::Error::custom(format!("invalid VRF proof {}", err)))?;
    Ok(output)
}

/// A consensus log item for BABE.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum BabeConsensusLog {
    NextEpochData,
    OnDisable,
    NextConfigData,
}

#[cfg(test)]
mod tests {
    use schnorrkel::vrf::VRFProof;

    use super::digest::PrimaryPreDigest;

    #[test]
    fn test_primary_se_and_de() {
        let mut sc1_and_sc2 = [0u8; 64];
        sc1_and_sc2[0] = 1;
        sc1_and_sc2[32] = 2;
        let vrf_proof = VRFProof::from_bytes(&sc1_and_sc2).expect("a vrf proof");

        let origin = PrimaryPreDigest {
            authority_index: 0,
            slot: 99.into(),
            vrf_output: Default::default(),
            vrf_proof: vrf_proof.to_bytes(),
        };

        let se_res = serde_json::to_string(&origin);
        assert!(se_res.is_ok());

        let de_res = serde_json::from_str::<PrimaryPreDigest>(se_res.unwrap().as_str());

        assert!(de_res.is_ok());

        let de_res = de_res.unwrap();
        assert_eq!(de_res.authority_index, origin.authority_index);
        assert_eq!(de_res.slot, origin.slot);
        assert_eq!(de_res.vrf_output, origin.vrf_output);
        assert_eq!(de_res.vrf_proof, origin.vrf_proof);
    }
}
