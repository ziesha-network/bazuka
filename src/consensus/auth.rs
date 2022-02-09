use std::sync::Arc;

use merlin::Transcript;
use primitive_types::U256;
use schnorrkel::vrf::VRFInOut;
use schnorrkel::PublicKey;

use crate::consensus::babe::Epoch;
use crate::consensus::header::{Authority, PreDigest, PrimaryDigest, SecondaryPlainDigest};
use crate::consensus::slots::Slot;
use crate::keystore::SyncCryptoApi;

pub const BABE_TRANSCRIPT: [u8; 4] = *b"BABE";
pub const BABE_VRF_PREFIX: &[u8] = b"babe-vrf";

pub(super) fn check_primary_threshold(inout: &VRFInOut, threshold: u128) -> bool {
    u128::from_le_bytes(inout.make_bytes::<[u8; 16]>(BABE_VRF_PREFIX)) < threshold
}

pub fn calculate_primary_threshold(
    c: (u64, u64),
    authorities: &[(Authority, u64)],
    authority_index: usize,
) -> u128 {
    use num_bigint::BigUint;
    use num_rational::BigRational;
    use num_traits::{cast::ToPrimitive, identities::One};

    let c = c.0 as f64 / c.1 as f64;

    let theta = authorities[authority_index].1 as f64
        / authorities.iter().map(|(_, weight)| weight).sum::<u64>() as f64;

    // NOTE: in the equation `p = 1 - (1 - c)^theta` the value of `p` is always
    // capped by `c`. For all pratical purposes `c` should always be set to a
    // value < 0.5, as such in the computations below we should never be near
    // edge cases like `0.999999`.

    let p = BigRational::from_float(1f64 - (1f64 - c).powf(theta)).expect(
        "returns None when the given value is not finite; \
		 c is a configuration parameter defined in (0, 1]; \
		 theta must be > 0 if the given authority's weight is > 0; \
		 theta represents the validator's relative weight defined in (0, 1]; \
		 powf will always return values in (0, 1] given both the \
		 base and exponent are in that domain; \
		 qed.",
    );

    let numer = p.numer().to_biguint().expect(
        "returns None when the given value is negative; \
		 p is defined as `1 - n` where n is defined in (0, 1]; \
		 p must be a value in [0, 1); \
		 qed.",
    );

    let denom = p.denom().to_biguint().expect(
        "returns None when the given value is negative; \
		 p is defined as `1 - n` where n is defined in (0, 1]; \
		 p must be a value in [0, 1); \
		 qed.",
    );

    ((BigUint::one() << 128) * numer / denom).to_u128().expect(
        "returns None if the underlying value cannot be represented with 128 bits; \
		 we start with 2^128 which is one more than can be represented with 128 bits; \
		 we multiple by p which is defined in [0, 1); \
		 the result must be lower than 2^128 by at least one and thus representable with 128 bits; \
		 qed.",
    )
}

pub(super) fn secondary_slot_author(
    slot: Slot,
    authorities: &[(AuthorityId, u64)],
    randomness: [u8; 32],
) -> Option<&Authority> {
    if authorities.is_empty() {
        return None;
    }

    let rand = U256::from((randomness, slot).using_encoded());

    let authorities_len = U256::from(authorities.len());
    let idx = rand % authorities_len;

    let expected_author = authorities.get(idx.as_u32() as usize).expect(
        "authorities not empty; index constrained to list length; \
				this is a valid index; qed",
    );

    Some(&expected_author.0)
}

pub fn claim_secondary_slot(
    slot: Slot,
    epoch: &Epoch,
    keys: &[(Authority, usize)],
    keystore: &Arc<dyn SyncCryptoApi>,
    author_secondary_vrf: bool,
) -> Option<(PreDigest, Authority)> {
    let Epoch {
        authorities,
        randomness,
        epoch_idx,
        ..
    } = epoch;

    if authorities.is_empty() {
        return None;
    }

    let expected_author = secondary_slot_author(slot, authorities, *randomness)?;

    for (authority_id, authority_index) in keys {
        if authority_id == expected_author {
            let pre_digest = if author_secondary_vrf {
                let mut transcript = Transcript::new(&BABE_TRANSCRIPT);
                transcript.append_u64(b"slot", slot.into());
                transcript.append_u64(b"epoch", *epoch_idx);
                transcript.append_message(b"randomness", randomness);

                // @TODO: sign transcript data
                let vrf_signature = SyncCryptoApi::vrf_sign(&**keystore);

                if let Ok(signature) = vrf_signature {
                    Some(PreDigest::SecondaryVRF(SecondaryVRFPreDigest {
                        slot,
                        vrf_output: signature.output.to_bytes(),
                        vrf_proof: signature.proof.to_bytes(),
                        authority_index: *authority_index as u32,
                    }))
                } else {
                    None
                }
            } else if SyncCryptoApi::has_keys(&**keystore, &authority_id.public_key) {
                Some(PreDigest::SecondaryPlain(SecondaryPlainDigest {
                    auth_idx: *authority_index as u32,
                    slot_num: slot,
                }))
            } else {
                None
            };

            if let Some(pre_digest) = pre_digest {
                return Some((pre_digest, authority_id.clone()));
            }
        }
    }
    None
}

pub fn claim_primary_slot(
    slot: Slot,
    epoch: &Epoch,
    c: (u64, u64),
    keystore: &Arc<dyn SyncCryptoApi>,
    keys: &[(Authority, usize)],
) -> Option<(PreDigest, Authority)> {
    let Epoch {
        authorities,
        randomness,
        epoch_idx,
        ..
    } = epoch;
    for (authority_id, authority_index) in keys {
        let mut transcript = Transcript::new(&BABE_TRANSCRIPT);
        transcript.append_u64(b"slot", slot.into());
        transcript.append_u64(b"epoch", *epoch_idx);
        transcript.append_message(b"randomness", randomness);

        let threshold = calculate_primary_threshold(c, &authorities[..], *authority_index);

        // @TODO
        let vrf_signature = SyncCryptoApi::vrf_sign(&**keystore);

        if let Ok(signature) = vrf_signature {
            let public = PublicKey::from_bytes(&authority_id.to_raw_vec()).ok()?;
            let inout = match signature.output.attach_input_hash(&public, transcript) {
                Ok(inout) => inout,
                Err(_) => continue,
            };

            if check_primary_threshold(&inout, threshold) {
                let pre_digest = PreDigest::Primary(PrimaryDigest {
                    auth_idx: *authority_index as u32,
                    vrf_output: signature.output.to_bytes(),
                    vrf_proof: signature.proof.to_bytes(),
                    slot_num: slot,
                });

                return Some((pre_digest, authority_id.clone()));
            }
        }
    }
    None
}

pub fn claim_slot_use_keys(
    slot: Slot,
    epoch: &Epoch,
    keystore: &Arc<dyn SyncCryptoApi>,
    keys: &[(Authority, usize)],
) -> Option<(PreDigest, Authority)> {
    let primary_slot = claim_primary_slot(slot, epoch, epoch.c, keystore, keys);
    primary_slot.or_else(|| {
        if epoch.allow_slots.is_secondary_plain_slots()
            || epoch.allow_slots.is_secondary_vrf_slots()
        {
            claim_secondary_slot(
                slot,
                &epoch,
                keys,
                keystore,
                epoch.allow_slots.is_secondary_vrf_slots(),
            )
        } else {
            None
        }
    })
}
