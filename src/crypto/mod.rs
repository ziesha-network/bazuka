use std::hash::Hash;

use generic_array::{ArrayLength, GenericArray};
use rand_core::{OsRng, RngCore};

pub use eddsa::*;
pub use mimc::*;

mod curve;
mod eddsa;
mod field;
mod mimc;
mod sr25519;

pub trait SignatureScheme {
    type Pub;
    type Priv;
    type Sig;
    fn generate_keys(seed: &[u8]) -> (Self::Pub, Self::Priv);
    fn sign(sk: &Self::Priv, msg: &[u8]) -> Self::Sig;
    fn verify(pk: &Self::Pub, msg: &[u8], sig: &Self::Sig) -> bool;
}

pub trait VerifiableRandomFunction {
    type Pub;
    type Priv;
    type Output;
    type Proof;
    fn generate(seed: &[u8]) -> (Self::Pub, Self::Priv);
    fn evaluate(sk: &Self::Priv, input: &[u8]) -> (Self::Output, Self::Proof);
    fn verify(pk: &Self::Pub, input: &[u8], output: &Self::Output, proof: &Self::Proof) -> bool;
}

pub trait CryptoCorrelation {
    type Pair: PairT;
}

pub trait PublicT:
    CryptoCorrelation
    + AsRef<[u8]>
    + AsMut<[u8]>
    + Default
    + PartialEq
    + Eq
    + Clone
    + Send
    + Sync
    + for<'a> TryFrom<&'a [u8]>
{
    /// A new instance from
    fn from_slice(data: &[u8]) -> Self;

    fn as_slice(&self) -> &[u8] {
        self.as_ref()
    }

    fn to_raw_vec(&self) -> Vec<u8> {
        self.as_slice().to_vec()
    }

    fn to_public(&self) -> CryptoIdWithPublic;
}

pub trait VRFPublic: PublicT {
    type VRFPair: VRFPair;

    fn vrf_verify(
        &self,
        data: VRFTranscriptData,
        sig: VRFSignature<
            <<Self as VRFPublic>::VRFPair as VRFPair>::OutputLen,
            <<Self as VRFPublic>::VRFPair as VRFPair>::ProofLen,
        >,
    ) -> Result<Vec<u8>, Error>;
}

pub trait VRFPair: PairT {
    type OutputLen: ArrayLength<u8>;
    type ProofLen: ArrayLength<u8>;
    type VRFPublic: PublicT + Hash + VRFPublic;

    fn vrf_sign(&self, data: VRFTranscriptData) -> VRFSignature<Self::OutputLen, Self::ProofLen>;
}

pub trait PairT: CryptoCorrelation + Sized + Clone + Send + Sync + 'static {
    type Public: PublicT + Hash;

    type Seed: Default + AsRef<[u8]> + AsMut<[u8]> + Clone;

    type Signature: AsRef<[u8]> + CryptoCorrelation;

    /// generate a ephemeral instance, since you won't have access to the secret key to storage
    fn generate() -> (Self, Self::Seed) {
        let mut seed = Self::Seed::default();
        OsRng.fill_bytes(seed.as_mut());
        (Self::from_seed(&seed), seed)
    }

    /// generate new secure (random) key pair and provide the recovery phrase.
    fn generate_with_phrase(password: Option<&str>) -> (Self, String, Self::Seed);

    /// generate new one from the English BIP39 seed `phrase`, or `None` if it's invalid.
    fn from_phrase(phrase: &str, password: Option<&str>) -> Result<(Self, Self::Seed), Error>;

    fn from_seed(seed: &Self::Seed) -> Self;

    fn from_seed_slice(seed: &[u8]) -> Result<Self, Error>;

    fn sign(&self, message: &[u8]) -> Self::Signature;

    fn verify<M: AsRef<[u8]>>(sig: &Self::Signature, message: M, pubkey: &Self::Public) -> bool;
}

///
pub struct VRFSignature<O: ArrayLength<u8>, P: ArrayLength<u8>> {
    pub output: GenericArray<u8, O>,
    pub proof: GenericArray<u8, P>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum VRFTranscriptValue {
    /// Value is an array of bytes
    Bytes(Vec<u8>),
    /// Value is a u64 integer
    U64(u64),
}

#[derive(Clone)]
pub struct VRFTranscriptData {
    pub label: &'static [u8],
    pub items: Vec<(&'static str, VRFTranscriptValue)>,
}

#[derive(
    Debug,
    Copy,
    Clone,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct CryptoId(pub [u8; 4]);

#[derive(
    Debug,
    Clone,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct CryptoIdWithPublic(pub CryptoId, pub Vec<u8>);

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("vrf error: {0}")]
    VRFSignatureError(schnorrkel::errors::SignatureError),
    #[error("the seed is invalid")]
    InvalidSeed,
    #[error("the {0} has an invalid length")]
    InvalidLength(String),
    #[error("{0}")]
    Custom(String),
    #[error("The seed phrase provided is not a valid BIP39 phrase")]
    InvalidPhrase,
}
