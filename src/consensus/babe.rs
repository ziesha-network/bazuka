use std::collections::HashMap;
use std::num::NonZeroU64;
use std::time::Duration;

use futures_timer::Delay;
use num_bigint::BigUint;
use num_rational::BigRational;
use num_traits::{One, Zero};
use schnorrkel::vrf::VRFInOut;

use crate::consensus::digest::{
    PreDigest, PrimaryPreDigest, SecondaryPlainPreDigest, SecondaryVRFPreDigest,
};
use crate::consensus::epoch::Epoch;
use crate::consensus::slots::{
    proposing_remaining_duration, Slot, SlotLenienceType, SlotProportion, Ticker,
};
use crate::consensus::{ChainSelector, CreateSlotAuxProvider, EpochBuilder};
use crate::consensus::{Error, Result};
use crate::core::digest::*;
use crate::core::Header;
use crate::crypto::{PublicKey, VRFPair, VRFPublicKey, VRFTranscript, VRFTranscriptData};

pub async fn start_babe_worker<ADP, CS, EPB>(
    slot_duration: Duration,
    aux_data_provider: ADP,
    chain_selector: CS,
    epoch_builder: EPB,
    block_proposal_slot_portion: &SlotProportion,
    max_block_proposal_slot_portion: Option<&SlotProportion>,
) where
    ADP: CreateSlotAuxProvider,
    CS: ChainSelector,
    EPB: EpochBuilder,
{
    let mut ticker = Ticker::new(slot_duration, aux_data_provider, chain_selector);

    loop {
        let slot_info = match ticker.next_slot().await {
            Ok(info) => info,
            Err(err) => {
                log::error!(target: "slots", "failed to poll next slot {:?}", err);
                return;
            }
        };
        let parent_slot = find_pre_digest(&slot_info.chain_head)
            .ok()
            .map(|head| head.slot());
        let remain = proposing_remaining_duration(
            parent_slot,
            &slot_info,
            block_proposal_slot_portion,
            max_block_proposal_slot_portion,
            SlotLenienceType::Exponential,
        );

        let proposing_remaining = if remain == Duration::default() {
            log::debug!(
                target: "slots",
                "Skipping proposal slot {} since there's no time left to propose", &slot_info.slot,
            );
            continue;
        } else {
            Delay::new(remain)
        };
        let (timestamp, slot) = (slot_info.timestamp, slot_info.slot);
        let epoch = match epoch_builder.best_epoch::<VRFPublicKey>(&slot_info.chain_head, slot) {
            Ok(epoch) => epoch,
            Err(e) => {
                log::error!(target: "slots", "Unable to build a best epoch at block {:?}, error: {:?}", slot_info.chain_head.hash(), e);
                continue;
            }
        };
        // @TODO: complete vrf pairs via public key
        let pairs = HashMap::new();
        let claim = match claim(slot, &epoch, epoch.c, &pairs) {
            None => {
                continue;
            }
            Some(claim) => claim,
        };
        let mut logs = Digests::default();
        logs.push(Digest::PreDigest(claim));
        // @TODO: propose log and use proposing_remaining as a deadline
    }
}

pub fn find_pre_digest(header: &Header) -> Result<PreDigest> {
    if header.number.is_zero() {
        return Ok(PreDigest::SecondaryPlain(SecondaryPlainPreDigest {
            authority_index: 0,
            slot: Default::default(),
        }));
    }
    let mut pre_digest: Option<_> = None;
    for log in header.logs() {
        match (log, pre_digest.is_some()) {
            (Digest::PreDigest(_), true) => return Err(Error::MultiplePreDigests),
            (Digest::PreDigest(pre), false) => pre_digest = Some(pre.clone()),
            _ => {}
        }
    }
    pre_digest.ok_or_else(|| Error::NoPreDigest)
}

#[derive(Debug, Clone)]
pub struct Authority<T: AsRef<[u8]>> {
    pub weight: NonZeroU64,
    pub public_key: T,
}

fn claim<P: PublicKey>(
    slot: Slot,
    epoch: &Epoch<P>,
    c: (u64, u64),
    pairs: &HashMap<usize, VRFPair>,
) -> Option<PreDigest> {
    claim_primary_slot(slot, epoch, c, pairs).or_else(|| claim_secondary_slot(slot, epoch, pairs))
}

fn claim_primary_slot<P: PublicKey>(
    slot: Slot,
    epoch: &Epoch<P>,
    c: (u64, u64),
    pairs: &HashMap<usize, VRFPair>,
) -> Option<PreDigest> {
    let Epoch {
        authorities,
        randomness,
        index,
        ..
    } = epoch;
    let weights = authorities
        .iter()
        .map(|author| author.weight.get())
        .collect::<Vec<u64>>();
    let transcript = make_vrf_transcript(*index, slot, randomness);
    for (idx, authority) in authorities.into_iter().enumerate() {
        let pair = match pairs.get(&idx) {
            None => {
                log::warn!(
                    "there's none pair which was matched with {}",
                    authority.public_key
                );
                continue;
            }
            Some(pair) => pair,
        };
        let signature = pair.vrf_sign(transcript.clone());
        let inout = match signature.attach_input_hash(&pair.to_public(), transcript.clone()) {
            Ok(inout) => inout,
            Err(err) => {
                log::error!(
                    "failed to attach the VRF signature with input hash, error: {}",
                    err
                );
                continue;
            }
        };
        let threshold = calculate_primary_threshold(c, idx, &weights);
        if check_primary_threshold(&inout, threshold) {
            let pre_digest = PreDigest::Primary(PrimaryPreDigest {
                slot,
                vrf_output: signature.output,
                vrf_proof: signature.proof,
                authority_index: idx as u32,
            });
            return Some(pre_digest);
        }
    }
    None
}

fn claim_secondary_slot<P: PublicKey>(
    slot: Slot,
    epoch: &Epoch<P>,
    pairs: &HashMap<usize, VRFPair>,
) -> Option<PreDigest> {
    let Epoch {
        authorities,
        randomness,
        index,
        ..
    } = epoch;
    if authorities.is_empty() {
        return None;
    }
    // @TODO: this is a fake round robin
    let r = (slot.0 / authorities.len() as u64) as usize;
    let transcript = make_vrf_transcript(*index, slot, randomness);
    pairs.get(&r).map(|pair| {
        let signature = pair.vrf_sign(transcript.clone());
        PreDigest::SecondaryVRF(SecondaryVRFPreDigest {
            slot,
            vrf_output: signature.output,
            vrf_proof: signature.proof,
            authority_index: r as u32,
        })
    })
}

fn check_primary_threshold(inout: &VRFInOut, threshold: BigUint) -> bool {
    BigUint::from_bytes_le(&inout.make_bytes::<[u8; 16]>(b"bazuka-baba")) < threshold
}

/// p = 1 - (1 - c)^theta
fn calculate_primary_threshold(c: (u64, u64), index: usize, weights: &[u64]) -> BigUint {
    let c = c.0 as f64 / c.1 as f64;

    let theta = weights[index] as f64 / weights.iter().map(|weight| weight).sum::<u64>() as f64;

    assert!(theta > 0.0, "authority with weight 0.");
    let p = BigRational::from_float(1f64 - (1f64 - c).powf(theta))
        .expect("given value must finite, p = 1 - (1 - c)^theta");
    let numer = p
        .numer()
        .to_biguint()
        .expect("numer can be extract from p which it must be a value in [0, 1)");
    let denom = p
        .denom()
        .to_biguint()
        .expect("denom can be extract from p which it must be a value in [0, 1)");
    (BigUint::one() << 128) * numer / denom
}

fn make_vrf_transcript(epoch_num: u64, slot: Slot, randomness: &[u8]) -> VRFTranscript {
    VRFTranscript {
        label: b"babe",
        messages: vec![
            (b"epoch number", VRFTranscriptData::U64(epoch_num)),
            (b"slot number", VRFTranscriptData::U64(slot.into())),
            (
                b"chain randomness",
                VRFTranscriptData::Bytes(randomness.to_vec()),
            ),
        ],
    }
}
