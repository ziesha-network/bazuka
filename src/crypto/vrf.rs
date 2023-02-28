use super::VerifiableRandomFunction;
use rand::Rng;
use rand::SeedableRng;
use rand::{CryptoRng, RngCore};
use rand_chacha::ChaChaRng;
use schnorrkel::Keypair;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct VRF;

#[derive(Clone)]
pub struct PrivateKey(pub schnorrkel::keys::SecretKey);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PublicKey(pub schnorrkel::keys::PublicKey);

impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

#[derive(Error, Debug)]
pub enum ParsePublicKeyError {
    #[error("vrf public key invalid")]
    Invalid,
}

impl std::fmt::Display for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "vrf")?;
        for byte in self.0.to_bytes().iter() {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

impl FromStr for PublicKey {
    type Err = ParsePublicKeyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with("vrf") || s.len() != 67 {
            return Err(ParsePublicKeyError::Invalid);
        }
        let bytes = hex::decode(&s[3..]).map_err(|_| ParsePublicKeyError::Invalid)?;
        Ok(Self(
            schnorrkel::keys::PublicKey::from_bytes(&bytes)
                .map_err(|_| ParsePublicKeyError::Invalid)?,
        ))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Proof(pub schnorrkel::vrf::VRFProof);
impl PartialEq<Proof> for Proof {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bytes() == other.0.to_bytes()
    }
}
impl Eq for Proof {}
impl std::hash::Hash for Proof {
    fn hash<Hasher>(&self, state: &mut Hasher)
    where
        Hasher: std::hash::Hasher,
    {
        state.write(&self.0.to_bytes());
        state.finish();
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Output(pub schnorrkel::vrf::VRFPreOut);
impl PartialEq<Output> for Output {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bytes() == other.0.to_bytes()
    }
}
impl Eq for Output {}
impl std::hash::Hash for Output {
    fn hash<Hasher>(&self, state: &mut Hasher)
    where
        Hasher: std::hash::Hasher,
    {
        state.write(&self.0.to_bytes());
        state.finish();
    }
}

impl Into<f32> for Output {
    fn into(self) -> f32 {
        ChaChaRng::from_seed(self.0 .0).gen_range(0.0..1.0)
    }
}

const SIGNING_CONTEXT: &'static [u8] = b"ZieshaVRF";

impl VerifiableRandomFunction for VRF {
    type Pub = PublicKey;
    type Priv = PrivateKey;
    type Proof = Proof;
    type Out = Output;
    fn generate_keys<R: CryptoRng + RngCore>(csprng: R) -> (PublicKey, PrivateKey) {
        let keypair: Keypair = Keypair::generate_with(csprng);
        (
            PublicKey(keypair.public.clone()),
            PrivateKey(keypair.secret.clone()),
        )
    }
    fn sign(sk: &PrivateKey, message: &[u8]) -> (Output, Proof) {
        let keypair = sk.0.clone().to_keypair();
        let ctx = schnorrkel::context::signing_context(SIGNING_CONTEXT);
        let (in_out, proof, _) = keypair.vrf_sign(ctx.bytes(message));
        (Output(in_out.to_preout()), Proof(proof))
    }
    fn verify(pk: &PublicKey, message: &[u8], out: &Output, proof: &Proof) -> bool {
        let ctx = schnorrkel::context::signing_context(SIGNING_CONTEXT);
        pk.0.vrf_verify(ctx.bytes(message), &out.0, &proof.0)
            .is_ok()
    }
}
