use std::ops::Deref;

use bip39::{Language, Mnemonic, MnemonicType};
use merlin::Transcript;
use schnorrkel::keys::{MINI_SECRET_KEY_LENGTH, SECRET_KEY_LENGTH};
use schnorrkel::vrf::VRF_PROOF_LENGTH;
use schnorrkel::{
    signing_context, ExpansionMode, MiniSecretKey, PublicKey, SecretKey, PUBLIC_KEY_LENGTH,
};
use typenum::{Unsigned, U32, U64};

use crate::core::bip39::mini_secret_from_entropy;
use crate::crypto::{
    CryptoCorrelation, CryptoIdWithPublic, PairT, PublicT, VRFPair, VRFPublic, VRFSignature,
    VRFTranscriptData, VRFTranscriptValue,
};
use crate::crypto::{CryptoId, Error};

const SIGNING_CTX: &[u8] = b"sr25_123_0990";
const SR25519_OUTPUT_LEN: usize = PUBLIC_KEY_LENGTH;
const SR25519_PROOF_LEN: usize = VRF_PROOF_LENGTH;

pub const CRYPTO_ID: CryptoId = CryptoId(*b"sr25");

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Default, Hash)]
pub struct Public(pub [u8; PUBLIC_KEY_LENGTH]);

impl VRFPublic for Public {
    type VRFPair = Pair;

    fn vrf_verify(
        &self,
        data: VRFTranscriptData,
        sig: VRFSignature<
            <<Self as VRFPublic>::VRFPair as VRFPair>::OutputLen,
            <<Self as VRFPublic>::VRFPair as VRFPair>::ProofLen,
        >,
    ) -> Result<Vec<u8>, Error> {
        let transcript = make_transcript(&data);
        let output = schnorrkel::vrf::VRFOutput(
            sig.output
                .as_slice()
                .try_into()
                .expect("VRFOutput with incorrect length"),
        );
        let proof = schnorrkel::vrf::VRFProof::from_bytes(&sig.proof)
            .map_err(|e| Error::VRFSignatureError(e))?;
        let public_key =
            schnorrkel::PublicKey::from_bytes(&self.0).map_err(|e| Error::VRFSignatureError(e))?;
        let (inout, _) = public_key
            .vrf_verify(transcript, &output, &proof)
            .map_err(|e| Error::VRFSignatureError(e))?;
        Ok(inout.to_output().to_bytes().to_vec())
    }
}

impl PublicT for Public {
    fn from_slice(data: &[u8]) -> Self {
        let mut r = [0u8; 32];
        r.copy_from_slice(data);
        Public(r)
    }

    fn to_public(&self) -> CryptoIdWithPublic {
        CryptoIdWithPublic(CRYPTO_ID, self.to_raw_vec())
    }
}

impl AsRef<[u8]> for Public {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsMut<[u8]> for Public {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0[..]
    }
}

impl Deref for Public {
    type Target = [u8; 32];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Public> for [u8; 32] {
    fn from(x: Public) -> [u8; 32] {
        x.0
    }
}

impl core::convert::TryFrom<&[u8]> for Public {
    type Error = Error;

    fn try_from(data: &[u8]) -> Result<Self, Error> {
        if data.len() == 32 {
            let mut inner = [0u8; 32];
            inner.copy_from_slice(data);
            Ok(Public(inner))
        } else {
            Err(Error::InvalidLength("sr25519's public".to_string()))
        }
    }
}

/// Schnorrkel/Ristretto "sr25519" key pair.
pub struct Pair(schnorrkel::Keypair);

impl VRFPair for Pair {
    type OutputLen = U32;
    type ProofLen = U64;
    type VRFPublic = Public;

    fn vrf_sign(&self, data: VRFTranscriptData) -> VRFSignature<Self::OutputLen, Self::ProofLen> {
        let transcript = make_transcript(&data);
        let (inout, proof, _) = self.0.vrf_sign(transcript);

        VRFSignature {
            output: inout
                .to_output()
                .to_bytes()
                .try_into()
                .expect("SR25519 vrfout is a 32 length array"),
            proof: proof
                .to_bytes()
                .try_into()
                .expect("SR25519 proof is a 64 length array"),
        }
    }
}

impl Clone for Pair {
    fn clone(&self) -> Self {
        Pair(schnorrkel::Keypair {
            public: self.0.public,
            secret: schnorrkel::SecretKey::from_bytes(&self.0.secret.to_bytes()[..])
                .expect("key is always the correct size"),
        })
    }
}

impl From<MiniSecretKey> for Pair {
    fn from(sec: MiniSecretKey) -> Pair {
        Pair(sec.expand_to_keypair(ExpansionMode::Ed25519))
    }
}

impl From<SecretKey> for Pair {
    fn from(sec: SecretKey) -> Pair {
        Pair(schnorrkel::Keypair::from(sec))
    }
}

impl From<schnorrkel::Keypair> for Pair {
    fn from(p: schnorrkel::Keypair) -> Pair {
        Pair(p)
    }
}

impl From<Pair> for schnorrkel::Keypair {
    fn from(p: Pair) -> schnorrkel::Keypair {
        p.0
    }
}

impl AsRef<schnorrkel::Keypair> for Pair {
    fn as_ref(&self) -> &schnorrkel::Keypair {
        &self.0
    }
}

impl Pair {
    pub fn from_entropy(entropy: &[u8], password: Option<&str>) -> (Pair, Seed) {
        let mini_key: MiniSecretKey = mini_secret_from_entropy(entropy, password.unwrap_or(""))
            .expect("32 bytes can always build a key;");

        let kp = mini_key.expand_to_keypair(ExpansionMode::Ed25519);
        (Pair(kp), mini_key.to_bytes())
    }

    pub fn from_seed_slice(seed: &[u8]) -> Result<Self, Error> {
        match seed.len() {
            MINI_SECRET_KEY_LENGTH => Ok(Pair(
                MiniSecretKey::from_bytes(seed)
                    .map_err(|_| Error::InvalidSeed)?
                    .expand_to_keypair(ExpansionMode::Ed25519),
            )),
            SECRET_KEY_LENGTH => Ok(Pair(
                SecretKey::from_bytes(seed)
                    .map_err(|_| Error::InvalidSeed)?
                    .to_keypair(),
            )),
            _ => Err(Error::InvalidLength("seed".to_string())),
        }
    }

    pub fn to_public(&self) -> Public {
        Public(self.0.public.to_bytes())
    }
}

impl PairT for Pair {
    type Public = Public;
    type Seed = Seed;
    type Signature = Signature;

    fn generate_with_phrase(password: Option<&str>) -> (Self, String, Self::Seed) {
        let mnemonic = Mnemonic::new(MnemonicType::Words12, Language::English);
        let phrase = mnemonic.phrase();
        let (pair, seed) = Self::from_phrase(phrase, password)
            .expect("All phrases generated by Mnemonic are valid; qed");
        (pair, phrase.to_owned(), seed)
    }

    fn from_phrase(phrase: &str, password: Option<&str>) -> Result<(Self, Self::Seed), Error> {
        Mnemonic::from_phrase(phrase, Language::English)
            .map_err(|_| Error::InvalidPhrase)
            .map(|m| Self::from_entropy(m.entropy(), password))
    }

    fn from_seed(seed: &Self::Seed) -> Self {
        Self::from_seed_slice(&seed[..]).expect("32 bytes can always build a key;")
    }

    fn from_seed_slice(seed: &[u8]) -> Result<Self, Error> {
        Self::from_seed_slice(seed)
    }

    fn sign(&self, message: &[u8]) -> Self::Signature {
        let context = signing_context(SIGNING_CTX);
        self.0.sign(context.bytes(message)).into()
    }

    fn verify<M: AsRef<[u8]>>(sig: &Self::Signature, message: M, pubkey: &Self::Public) -> bool {
        let signature = match schnorrkel::Signature::from_bytes(&sig.0) {
            Ok(signature) => signature,
            Err(_) => return false,
        };

        let pub_key = match PublicKey::from_bytes(pubkey.as_ref()) {
            Ok(pub_key) => pub_key,
            Err(_) => return false,
        };
        pub_key
            .verify_simple(SIGNING_CTX, message.as_ref(), &signature)
            .is_ok()
    }
}

type Seed = [u8; MINI_SECRET_KEY_LENGTH];

pub struct Signature(pub [u8; 64]);

impl Clone for Signature {
    fn clone(&self) -> Self {
        let mut r = [0u8; 64];
        r.copy_from_slice(&self.0[..]);
        Signature(r)
    }
}

impl core::convert::TryFrom<&[u8]> for Signature {
    type Error = ();

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        if data.len() == 64 {
            let mut inner = [0u8; 64];
            inner.copy_from_slice(data);
            Ok(Signature(inner))
        } else {
            Err(())
        }
    }
}

impl serde::Serialize for Signature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&hex::encode(self))
    }
}

impl<'de> serde::Deserialize<'de> for Signature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let signature_hex = hex::decode(&String::deserialize(deserializer)?)
            .map_err(|e| serde::de::Error::custom(format!("{:?}", e)))?;
        Signature::try_from(signature_hex.as_ref())
            .map_err(|e| serde::de::Error::custom(format!("{:?}", e)))
    }
}

impl Default for Signature {
    fn default() -> Self {
        Signature([0u8; 64])
    }
}

impl PartialEq for Signature {
    fn eq(&self, b: &Self) -> bool {
        self.0[..] == b.0[..]
    }
}

impl Eq for Signature {}

impl From<Signature> for [u8; 64] {
    fn from(v: Signature) -> [u8; 64] {
        v.0
    }
}

impl AsRef<[u8; 64]> for Signature {
    fn as_ref(&self) -> &[u8; 64] {
        &self.0
    }
}

impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
    }
}

impl AsMut<[u8]> for Signature {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0[..]
    }
}

impl From<schnorrkel::Signature> for Signature {
    fn from(s: schnorrkel::Signature) -> Signature {
        Signature(s.to_bytes())
    }
}

impl std::hash::Hash for Signature {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.0[..], state);
    }
}

impl CryptoCorrelation for Pair {
    type Pair = Pair;
}

impl CryptoCorrelation for Public {
    type Pair = Pair;
}

impl CryptoCorrelation for Signature {
    type Pair = Pair;
}

/// convert to `merlin Transcript`
pub fn make_transcript(data: &VRFTranscriptData) -> Transcript {
    let mut transcript = Transcript::new(data.label);
    for (label, value) in data.items.iter() {
        match value {
            VRFTranscriptValue::Bytes(ref bytes) => {
                transcript.append_message((*label).as_bytes(), &bytes);
            }
            VRFTranscriptValue::U64(ref val) => {
                transcript.append_u64((*label).as_bytes(), *val);
            }
        }
    }
    transcript
}

#[cfg(test)]
mod tests {
    use crate::crypto::sr25519::{Pair, VRFTranscriptData, VRFTranscriptValue};
    use crate::crypto::{PairT, VRFPair, VRFPublic};

    #[test]
    fn sign_and_verify() {
        let pair = Pair::from_seed_slice(b"12345678901234567890123456789012")
            .expect("create sr25519 pair");
        let sig = pair.sign(b"friendly");
        let public = pair.to_public();
        assert!(Pair::verify(&sig, b"friendly", &public));

        assert!(!Pair::verify(&sig, b"not friendly", &public));
    }

    #[test]
    fn vrf_test_ok() {
        let pair = Pair::from_seed_slice(b"12345678901234567890123456789012")
            .expect("create sr25519 pair");
        let sig = pair.vrf_sign(VRFTranscriptData {
            label: b"a label",
            items: vec![
                ("one", VRFTranscriptValue::U64(1)),
                ("two", VRFTranscriptValue::Bytes("two".as_bytes().to_vec())),
            ],
        });
        assert!(pair
            .to_public()
            .vrf_verify(
                VRFTranscriptData {
                    label: b"a label",
                    items: vec![
                        ("one", VRFTranscriptValue::U64(1)),
                        ("two", VRFTranscriptValue::Bytes("two".as_bytes().to_vec())),
                    ],
                },
                sig,
            )
            .is_ok())
    }

    #[test]
    fn vrf_test_not_ok() {
        let pair = Pair::from_seed_slice(b"12345678901234567890123456789012")
            .expect("create sr25519 pair");
        let sig = pair.vrf_sign(VRFTranscriptData {
            label: b"alice",
            items: vec![
                ("one", VRFTranscriptValue::U64(1)),
                (
                    "now alice has one",
                    VRFTranscriptValue::Bytes("now alice has one".as_bytes().to_vec()),
                ),
            ],
        });
        assert!(pair
            .to_public()
            .vrf_verify(
                VRFTranscriptData {
                    label: b"fake label",
                    items: vec![
                        ("two", VRFTranscriptValue::U64(2)),
                        (
                            "now alice has two",
                            VRFTranscriptValue::Bytes("now alice has two".as_bytes().to_vec()),
                        ),
                    ],
                },
                sig,
            )
            .is_err())
    }
}
