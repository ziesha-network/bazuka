use std::fmt::{Display, Formatter};

use schnorrkel::keys::{MINI_SECRET_KEY_LENGTH, SECRET_KEY_LENGTH};
use schnorrkel::vrf::{VRFInOut, VRFOutput, VRFProof, VRF_OUTPUT_LENGTH, VRF_PROOF_LENGTH};
use schnorrkel::{ExpansionMode, MiniSecretKey, SecretKey, SignatureResult};

use crate::crypto::{Error, VRFTranscript, VRFTranscriptData, VerifiableRandomFunction};

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
        output: &[u8; VRF_OUTPUT_LENGTH],
        proof: &[u8; VRF_PROOF_LENGTH],
    ) -> Result<Vec<u8>, Error> {
        let transcript = to_transcript(transcript);
        let output = VRFOutput::from_bytes(output)
            .map_err(|e| Error::VRFSignatureError(format!("{}", e)))?;
        let proof =
            VRFProof::from_bytes(proof).map_err(|e| Error::VRFSignatureError(format!("{}", e)))?;
        let (inout, _) = self
            .0
            .vrf_verify(transcript, &output, &proof)
            .map_err(|e| Error::VRFSignatureError(format!("{}", e)))?;
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

pub fn to_transcript(t: VRFTranscript) -> merlin::Transcript {
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

impl VerifiableRandomFunction for VRFPair {
    type Pub = VRFPublicKey;
    type Output = [u8; VRF_OUTPUT_LENGTH];
    type Proof = [u8; VRF_PROOF_LENGTH];

    fn generate(seed: &[u8]) -> Result<Self, Error> {
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

    fn sign(&self, transcript: VRFTranscript) -> (Self::Output, Self::Proof) {
        let (inout, proof, _) = self.0.vrf_sign(to_transcript(transcript));
        (inout.to_output().to_bytes(), proof.to_bytes())
    }

    fn verify(
        public_key: &Self::Pub,
        transcript: VRFTranscript,
        output: Self::Output,
        proof: Self::Proof,
    ) -> Result<Vec<u8>, Error> {
        public_key.vrf_verify(transcript, &output, &proof)
    }
}

#[cfg(test)]
mod test {
    use crate::crypto::{VRFPair, VRFTranscript, VRFTranscriptData, VerifiableRandomFunction};

    #[test]
    fn vrf_test_ok() {
        let pair =
            VRFPair::generate(b"12345678901234567890123456789012").expect("create sr25519 pair");
        let sig = pair.sign(VRFTranscript {
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
                &sig.0,
                &sig.1,
            )
            .is_ok())
    }

    #[test]
    fn vrf_test_not_ok() {
        let pair =
            VRFPair::generate(b"12345678901234567890123456789012").expect("create sr25519 pair");
        let sig = pair.sign(VRFTranscript {
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
                &sig.0,
                &sig.1,
            )
            .is_err())
    }
}
