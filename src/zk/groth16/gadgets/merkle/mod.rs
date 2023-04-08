use crate::zk::ZkScalar;
use ff::Field;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Proof<const LOG4_TREE_SIZE: u8>(pub Vec<[ZkScalar; 3]>);

impl<const LOG4_TREE_SIZE: u8> Default for Proof<LOG4_TREE_SIZE> {
    fn default() -> Self {
        Self(vec![[ZkScalar::ZERO; 3]; LOG4_TREE_SIZE as usize])
    }
}

use super::common::UnsignedInteger;
use super::common::{boolean_or, Number};
use super::{common, poseidon};
use crate::zk::groth16::gadgets::BellmanFr;
use bellman::gadgets::boolean::{AllocatedBit, Boolean};
use bellman::gadgets::num::AllocatedNum;
use bellman::{ConstraintSystem, SynthesisError};

fn merge_hash_poseidon4<CS: ConstraintSystem<BellmanFr>>(
    cs: &mut CS,
    select: (&AllocatedBit, &AllocatedBit),
    v: &Number,
    p: &[AllocatedNum<BellmanFr>; 3],
) -> Result<Number, SynthesisError> {
    let select = (Boolean::Is(select.0.clone()), Boolean::Is(select.1.clone()));

    let and = Boolean::and(&mut *cs, &select.0, &select.1)?;
    let or = boolean_or(&mut *cs, &select.0, &select.1)?;

    // v0 == s0_or_s1 ? p[0] : v
    let v0 = common::mux(&mut *cs, &or, v, &p[0].clone().into())?;

    //v1p == s0 ? v : p[0]
    let v1p = common::mux(&mut *cs, &select.0, &p[0].clone().into(), v)?;

    //v1 == s1 ? p[1] : v1p
    let v1 = common::mux(&mut *cs, &select.1, &v1p.into(), &p[1].clone().into())?;

    //v2p == s0 ? p[2] : v
    let v2p = common::mux(&mut *cs, &select.0, v, &p[2].clone().into())?;

    //v2 == s1 ? v2p : p[1]
    let v2 = common::mux(&mut *cs, &select.1, &p[1].clone().into(), &v2p.into())?;

    //v3 == s0_and_s1 ? v : p[2]
    let v3 = common::mux(&mut *cs, &and, &p[2].clone().into(), &v)?;

    poseidon::poseidon(cs, &[&v0.into(), &v1.into(), &v2.into(), &v3.into()])
}

pub fn calc_root_poseidon4<CS: ConstraintSystem<BellmanFr>>(
    cs: &mut CS,
    index: &UnsignedInteger,
    val: &Number,
    proof: &[[AllocatedNum<BellmanFr>; 3]],
) -> Result<Number, SynthesisError> {
    assert_eq!(index.bits().len(), proof.len() * 2);
    let mut curr = val.clone();
    for (p, dir) in proof.into_iter().zip(index.bits().chunks(2)) {
        curr = merge_hash_poseidon4(&mut *cs, (&dir[0], &dir[1]), &curr, p)?;
    }
    Ok(curr)
}

pub fn check_proof_poseidon4<CS: ConstraintSystem<BellmanFr>>(
    cs: &mut CS,
    enabled: &Boolean,
    index: &UnsignedInteger,
    val: &Number,
    proof: &[[AllocatedNum<BellmanFr>; 3]],
    root: &Number,
) -> Result<(), SynthesisError> {
    let new_root = calc_root_poseidon4(&mut *cs, index, val, proof)?;
    root.assert_equal_if_enabled(cs, enabled, &new_root)?;
    Ok(())
}

#[cfg(test)]
mod test;
