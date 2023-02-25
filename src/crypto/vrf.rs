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
pub struct Signature(pub schnorrkel::Signature);

#[derive(Clone)]
pub struct Context(pub schnorrkel::context::SigningContext);

impl VerifiableRandomFunction for VRF {
    type Ctx = Context;
    type Pub = PublicKey;
    type Priv = PrivateKey;
    type Sig = Signature;
    fn make_context(bytes: &[u8]) -> Context {
        Context(schnorrkel::context::signing_context(bytes))
    }
    fn generate_keys<R: CryptoRng + RngCore>(csprng: R) -> (PublicKey, PrivateKey) {
        let keypair: Keypair = Keypair::generate_with(csprng);
        (
            PublicKey(keypair.public.clone()),
            PrivateKey(keypair.secret.clone()),
        )
    }
    fn sign(sk: &PrivateKey, ctx: &Context, message: &[u8]) -> Signature {
        Signature(sk.0.clone().to_keypair().sign(ctx.0.bytes(message)))
    }
    fn verify(pk: &PublicKey, ctx: &Context, message: &[u8], sig: &Signature) -> bool {
        pk.0.verify(ctx.0.bytes(message), &sig.0).is_ok()
    }
}
