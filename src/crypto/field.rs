use ff::PrimeField;
use serde::{Deserialize, Serialize};

#[derive(PrimeField, Serialize, Deserialize)]
#[PrimeFieldModulus = "21888242871839275222246405745257275088548364400416034343698204186575808495617"]
#[PrimeFieldGenerator = "7"]
#[PrimeFieldReprEndianness = "little"]
pub struct Fr([u64; 4]);
