use core::fmt;
use std::fmt::Debug;

use schnorrkel::vrf::{VRFOutput, VRFProof};
use serde::de::{Error, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeSeq};
use serde::{Deserialize, Deserializer, Serializer};

use crate::consensus::slots::Slot;

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

#[derive(Clone, Debug)]
pub struct PrimaryPreDigest {
    pub authority_index: u32,
    pub slot: Slot,
    pub vrf_output: VRFOutput,
    pub vrf_proof: VRFProof,
}

impl PrimaryPreDigest {
    pub fn new(
        authority_index: u32,
        slot: Slot,
        vrf_output: VRFOutput,
        vrf_proof: VRFProof,
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

#[derive(Clone, Debug)]
pub struct SecondaryVRFPreDigest {
    pub authority_index: u32,
    pub slot: Slot,
    pub vrf_output: VRFOutput,
    pub vrf_proof: VRFProof,
}

impl SecondaryVRFPreDigest {
    pub fn new(
        authority_index: u32,
        slot: Slot,
        vrf_output: VRFOutput,
        vrf_proof: VRFProof,
    ) -> Self {
        Self {
            authority_index,
            slot,
            vrf_output,
            vrf_proof,
        }
    }
}

macro_rules! seq_deserialize {
    ( $var:tt, $typ:ty, $msg:expr$(,)? ) => {
        $var.next_element::<$typ>()
            .map_err(|err| A::Error::custom(format!($msg, err)))?
            .unwrap()
    };
}
/// may add a version control management u32 + u64 + 32 array + 64 array
#[macro_export]
macro_rules! se_de_primary_and_secondary_vrf {
    ($typ:ty, $init_func:ident) => {
        impl Serialize for $typ {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                let len = 4 + 8 + 32 + 64;
                let mut s = serializer.serialize_seq(None)?;
                s.serialize_element(&len)?;
                s.serialize_element(&self.authority_index)?;
                let slot: &u64 = &self.slot.into();
                s.serialize_element(slot)?;
                s.serialize_element(&self.vrf_output.as_bytes())?;

                let items: Result<Vec<_>, _> = self
                    .vrf_proof
                    .to_bytes()
                    .iter()
                    .map(|u| s.serialize_element(u))
                    .collect();
                match items {
                    Ok(_) => {}
                    Err(err) => return Err(err),
                }
                s.end()
            }
        }

        impl<'de> Deserialize<'de> for $typ {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct SeqVisitor<T> {
                    _phantom: std::marker::PhantomData<T>,
                }

                impl<'de> Visitor<'de> for SeqVisitor<$typ> {
                    type Value = $typ;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("`secs` or `nanos`")
                    }

                    fn visit_seq<A: SeqAccess<'de>>(
                        self,
                        mut seq: A,
                    ) -> Result<Self::Value, A::Error> {
                        let len = seq_deserialize!(
                            seq,
                            u32,
                            "a bytes format digest must has a length, {}",
                        );
                        let expect_len = 4 + 8 + 32 + 64;
                        if len != expect_len {
                            log::error!("The length of digest in byte form must be {}", expect_len);
                            return Err(A::Error::custom(format!(
                                "The length of digest in byte form must be {}",
                                expect_len
                            )));
                        }
                        let authority_index = seq_deserialize!(
                            seq,
                            u32,
                            "must has authority_index when deserializing, {}",
                        );

                        let slot: Slot =
                            seq_deserialize!(seq, u64, "must has slot when deserializing, {}")
                                .into();
                        let vrf_output = seq_deserialize!(
                            seq,
                            [u8; 32],
                            "must has vrf_output when deserializing, {}",
                        );
                        let vrf_output = VRFOutput(vrf_output);
                        let mut vrf_proof = [0u8; 64];
                        for i in 0..64 {
                            vrf_proof[i] = seq
                                .next_element::<u8>()
                                .map_err(|err| {
                                    A::Error::custom(format!(
                                        "must has vrf_proof when deserializing, {}",
                                        err
                                    ))
                                })?
                                .unwrap();
                        }
                        let vrf_proof = VRFProof::from_bytes(&vrf_proof).map_err(|err| {
                            A::Error::custom(format!("invalid VRF proof {}", err))
                        })?;

                        Ok(<$typ>::$init_func(
                            authority_index,
                            slot,
                            vrf_output,
                            vrf_proof,
                        ))
                    }
                }
                deserializer.deserialize_any(SeqVisitor {
                    _phantom: Default::default(),
                })
            }
        }
    };
}

se_de_primary_and_secondary_vrf!(PrimaryPreDigest, new);
se_de_primary_and_secondary_vrf!(SecondaryVRFPreDigest, new);

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

    use crate::consensus::digest::PrimaryPreDigest;

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
            vrf_proof,
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
