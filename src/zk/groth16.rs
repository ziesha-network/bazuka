use super::ZkScalar;
use bls12_381::{Bls12, G1Affine as BellmanG1, G2Affine as BellmanG2, Scalar as BellmanFr};
use serde::{Deserialize, Serialize};

impl From<ZkScalar> for BellmanFr {
    fn from(s: ZkScalar) -> BellmanFr {
        unsafe { std::mem::transmute::<ZkScalar, BellmanFr>(s) }
    }
}

impl From<BellmanFr> for ZkScalar {
    fn from(bls: BellmanFr) -> ZkScalar {
        unsafe { std::mem::transmute::<BellmanFr, ZkScalar>(bls) }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Fp([u64; 6]);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Groth16VerifyingKey {
    alpha_g1: (Fp, Fp, bool),
    beta_g1: (Fp, Fp, bool),
    beta_g2: ((Fp, Fp), (Fp, Fp), bool),
    gamma_g2: ((Fp, Fp), (Fp, Fp), bool),
    delta_g1: (Fp, Fp, bool),
    delta_g2: ((Fp, Fp), (Fp, Fp), bool),
    ic: Vec<(Fp, Fp, bool)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Groth16Proof {
    a: (Fp, Fp, bool),
    b: ((Fp, Fp), (Fp, Fp), bool),
    c: (Fp, Fp, bool),
}

pub fn groth16_verify(
    vk: &Groth16VerifyingKey,
    prev_height: u64,
    prev_state: ZkScalar,
    aux_data: ZkScalar,
    next_state: ZkScalar,
    proof: &Groth16Proof,
) -> bool {
    let (vk, proof) = unsafe {
        let alpha_g1 = std::mem::transmute::<(Fp, Fp, bool), BellmanG1>(vk.alpha_g1.clone());
        let beta_g1 = std::mem::transmute::<(Fp, Fp, bool), BellmanG1>(vk.beta_g1.clone());
        let beta_g2 =
            std::mem::transmute::<((Fp, Fp), (Fp, Fp), bool), BellmanG2>(vk.beta_g2.clone());
        let gamma_g2 =
            std::mem::transmute::<((Fp, Fp), (Fp, Fp), bool), BellmanG2>(vk.gamma_g2.clone());
        let delta_g1 = std::mem::transmute::<(Fp, Fp, bool), BellmanG1>(vk.delta_g1.clone());
        let delta_g2 =
            std::mem::transmute::<((Fp, Fp), (Fp, Fp), bool), BellmanG2>(vk.delta_g2.clone());
        let ic = vk
            .ic
            .iter()
            .cloned()
            .map(|p| std::mem::transmute::<(Fp, Fp, bool), BellmanG1>(p))
            .collect();
        let proof = bellman::groth16::Proof::<Bls12> {
            a: std::mem::transmute::<(Fp, Fp, bool), BellmanG1>(proof.a.clone()),
            b: std::mem::transmute::<((Fp, Fp), (Fp, Fp), bool), BellmanG2>(proof.b.clone()),
            c: std::mem::transmute::<(Fp, Fp, bool), BellmanG1>(proof.c.clone()),
        };
        let vk =
            bellman::groth16::prepare_verifying_key(&bellman::groth16::VerifyingKey::<Bls12> {
                alpha_g1,
                beta_g1,
                beta_g2,
                gamma_g2,
                delta_g1,
                delta_g2,
                ic,
            });
        (vk, proof)
    };
    bellman::groth16::verify_proof(
        &vk,
        &proof,
        &[
            prev_height.into(),
            prev_state.into(),
            aux_data.into(),
            next_state.into(),
        ],
    )
    .is_ok()
}
