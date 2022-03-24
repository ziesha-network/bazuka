use std::fmt::{Display, Formatter};
use std::thread::sleep;

use schnorrkel::keys::{MINI_SECRET_KEY_LENGTH, SECRET_KEY_LENGTH};
use schnorrkel::vrf::{VRFInOut, VRFOutput, VRFProof};
use schnorrkel::{ExpansionMode, MiniSecretKey, SecretKey, SignatureResult};

use crate::crypto::{PublicKey, VerifiableRandomFunction};

pub struct VRFPublicKey(pub schnorrkel::keys::PublicKey);

impl AsRef<[u8]> for VRFPublicKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl Display for VRFPublicKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0.to_bytes()))
    }
}

impl VRFPublicKey {
    pub fn vrf_verify(
        &self,
        transcript: VRFTranscript,
        signature: VRFSignature,
    ) -> Result<Vec<u8>, Error> {
        let transcript = to_transcript(transcript);
        let (inout, _) = self
            .0
            .vrf_verify(transcript, &signature.output, &signature.proof)
            .map_err(|e| Error::VRFSignatureError(e))?;
        Ok(inout.to_output().to_bytes().to_vec())
    }
}

pub struct VRFPrivateKey(schnorrkel::keys::SecretKey);

pub struct VRFPair(schnorrkel::Keypair);

impl VRFPair {
    pub fn to_public(&self) -> VRFPublicKey {
        VRFPublicKey(self.0.public)
    }
}

pub struct VRFSignature {
    pub output: VRFOutput,
    pub proof: VRFProof,
}

impl VRFSignature {
    pub fn attach_input_hash(
        &self,
        public_key: &VRFPublicKey,
        transcript: VRFTranscript,
    ) -> SignatureResult<VRFInOut> {
        self.output
            .attach_input_hash(&public_key.0, to_transcript(transcript))
    }
}

#[derive(Clone)]
pub enum VRFTranscriptData {
    U64(u64),
    Bytes(Vec<u8>),
}

#[derive(Clone)]
pub struct VRFTranscript {
    pub label: &'static [u8],
    pub messages: Vec<(&'static [u8], VRFTranscriptData)>,
}

fn to_transcript(t: VRFTranscript) -> merlin::Transcript {
    let mut transcript = merlin::Transcript::new(t.label);
    for (label, data) in t.messages.into_iter() {
        match data {
            VRFTranscriptData::U64(u) => {
                transcript.append_u64(label, u);
            }
            VRFTranscriptData::Bytes(bytes) => {
                transcript.append_message(label, &bytes);
            }
        }
    }
    transcript
}

impl VRFPair {
    pub fn generate_random() -> Self {
        Self(schnorrkel::Keypair::generate())
    }

    pub fn generate(seed: &[u8]) -> Result<Self, Error> {
        match seed.len() {
            MINI_SECRET_KEY_LENGTH => Ok(VRFPair(
                MiniSecretKey::from_bytes(seed)
                    .map_err(|_| Error::InvalidSeed)?
                    .expand_to_keypair(ExpansionMode::Ed25519),
            )),
            SECRET_KEY_LENGTH => Ok(VRFPair(
                SecretKey::from_bytes(seed)
                    .map_err(|_| Error::InvalidSeed)?
                    .to_keypair(),
            )),
            _ => Err(Error::InvalidLength("seed".to_string())),
        }
    }

    pub fn vrf_sign(&self, transcript: VRFTranscript) -> VRFSignature {
        let (inout, proof, _) = self.0.vrf_sign(to_transcript(transcript));
        VRFSignature {
            output: inout.to_output(),
            proof,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("vrf error: {0}")]
    VRFSignatureError(schnorrkel::errors::SignatureError),
    #[error("the {0} has an invalid length")]
    InvalidLength(String),
    #[error("the seed is invalid")]
    InvalidSeed,
}

#[cfg(test)]
mod test {
    use crate::crypto::{VRFPair, VRFTranscript, VRFTranscriptData};

    #[test]
    fn vrf_test_ok() {
        let pair =
            VRFPair::generate(b"12345678901234567890123456789012").expect("create sr25519 pair");
        let sig = pair.vrf_sign(VRFTranscript {
            label: b"a label",
            messages: vec![
                (b"one", VRFTranscriptData::U64(1)),
                (b"two", VRFTranscriptData::Bytes("two".as_bytes().to_vec())),
            ],
        });
        assert!(pair
            .to_public()
            .vrf_verify(
                VRFTranscript {
                    label: b"a label",
                    messages: vec![
                        (b"one", VRFTranscriptData::U64(1)),
                        (b"two", VRFTranscriptData::Bytes("two".as_bytes().to_vec())),
                    ],
                },
                sig,
            )
            .is_ok())
    }

    #[test]
    fn vrf_test_not_ok() {
        let pair =
            VRFPair::generate(b"12345678901234567890123456789012").expect("create sr25519 pair");
        let sig = pair.vrf_sign(VRFTranscript {
            label: b"alice",
            messages: vec![
                (b"one", VRFTranscriptData::U64(1)),
                (
                    b"now alice has one",
                    VRFTranscriptData::Bytes("now alice has one".as_bytes().to_vec()),
                ),
            ],
        });
        assert!(pair
            .to_public()
            .vrf_verify(
                VRFTranscript {
                    label: b"fake label",
                    messages: vec![
                        (b"two", VRFTranscriptData::U64(2)),
                        (
                            b"now alice has two",
                            VRFTranscriptData::Bytes("now alice has two".as_bytes().to_vec()),
                        ),
                    ],
                },
                sig,
            )
            .is_err())
    }
}
