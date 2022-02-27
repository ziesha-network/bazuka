use crate::core::number::U256;
use ff::{Field, PrimeField};
use serde::{Deserialize, Serialize};

#[derive(PrimeField, Serialize, Deserialize)]
#[PrimeFieldModulus = "21888242871839275222246405745257275088548364400416034343698204186575808495617"]
#[PrimeFieldGenerator = "7"]
#[PrimeFieldReprEndianness = "little"]
pub struct Fr([u64; 4]);

impl Fr {
    pub fn from_u256(num: &U256) -> Self {
        let mut elem = Fr::zero();
        let bytes = num.to_bytes();
        elem.0[0] = u64::from_le_bytes(bytes[..8].try_into().unwrap());
        elem.0[1] = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
        elem.0[2] = u64::from_le_bytes(bytes[16..24].try_into().unwrap());
        elem.0[3] = u64::from_le_bytes(bytes[24..32].try_into().unwrap());
        elem
    }
}
