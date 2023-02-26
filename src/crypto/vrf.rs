use super::VerifiableRandomFunction;
use rand::SeedableRng;
use rand::{CryptoRng, RngCore};
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

impl FromStr for PublicKey {
    type Err = ParsePublicKeyError;
    fn from_str(_s: &str) -> Result<Self, Self::Err> {
        // TODO: Implement this!
        // WARN: Fix this!
        let keypair: Keypair =
            Keypair::generate_with(&mut rand_chacha::ChaChaRng::seed_from_u64(0));
        Ok(PublicKey(keypair.public.clone()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proof(pub schnorrkel::vrf::VRFProof);

#[derive(Clone)]
pub struct Output(pub schnorrkel::vrf::VRFPreOut);

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
