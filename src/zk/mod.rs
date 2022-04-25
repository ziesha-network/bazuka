mod mimc;
pub mod ram;

use crate::crypto::Fr;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ZkScalar(Fr);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ZkVerifierKey(#[serde(with = "serde_bytes")] Vec<u8>);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ZkProof(#[serde(with = "serde_bytes")] Vec<u8>);
