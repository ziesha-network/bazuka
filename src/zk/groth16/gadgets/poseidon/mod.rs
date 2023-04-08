use super::common::Number;
use crate::zk::groth16::gadgets::BellmanFr;

use crate::zk::poseidon::PoseidonParams;
use bellman::gadgets::num::AllocatedNum;
use bellman::{ConstraintSystem, SynthesisError};

fn sbox<CS: ConstraintSystem<BellmanFr>>(
    cs: &mut CS,
    a: &Number,
) -> Result<AllocatedNum<BellmanFr>, SynthesisError> {
    let a2 = a.mul(&mut *cs, &a)?;
    let a4 = a2.mul(&mut *cs, &a2)?;
    a.mul(&mut *cs, &a4.into())
}

fn add_constants<CS: ConstraintSystem<BellmanFr>>(
    vals: &mut [Number],
    const_offset: usize,
    params: &PoseidonParams,
) {
    for (i, val) in vals.iter_mut().enumerate() {
        val.add_constant::<CS>(params.round_constants[const_offset + i].into());
    }
}

fn partial_round<CS: ConstraintSystem<BellmanFr>>(
    cs: &mut CS,
    const_offset: usize,
    mut vals: Vec<Number>,
    params: &PoseidonParams,
) -> Result<Vec<Number>, SynthesisError> {
    add_constants::<CS>(&mut vals, const_offset, params);

    vals[0] = sbox(&mut *cs, &vals[0])?.into();
    for i in 1..vals.len() {
        vals[i] = vals[i].clone().compress(&mut *cs)?.into();
    }

    product_mds(vals, params)
}

fn full_round<CS: ConstraintSystem<BellmanFr>>(
    cs: &mut CS,
    const_offset: usize,
    mut vals: Vec<Number>,
    params: &PoseidonParams,
) -> Result<Vec<Number>, SynthesisError> {
    add_constants::<CS>(&mut vals, const_offset, params);

    for val in vals.iter_mut() {
        *val = sbox(&mut *cs, val)?.into();
    }

    product_mds(vals, params)
}

fn product_mds(vals: Vec<Number>, params: &PoseidonParams) -> Result<Vec<Number>, SynthesisError> {
    let mut result = vec![Number::zero(); vals.len()];
    for j in 0..vals.len() {
        for k in 0..vals.len() {
            let mat_val: BellmanFr = params.mds_constants[j][k].into();
            result[j] = result[j].clone() + (mat_val, vals[k].clone());
        }
    }
    Ok(result)
}

pub fn poseidon<CS: ConstraintSystem<BellmanFr>>(
    cs: &mut CS,
    vals: &[&Number],
) -> Result<Number, SynthesisError> {
    let mut elems = vals.iter().map(|v| (*v).clone()).collect::<Vec<Number>>();
    elems.insert(0, Number::zero());

    let params = PoseidonParams::for_width(elems.len()).unwrap();
    let mut const_offset = 0;

    for _ in 0..params.full_rounds / 2 {
        elems = full_round(&mut *cs, const_offset, elems, &params)?;
        const_offset += elems.len();
    }

    for _ in 0..params.partial_rounds {
        elems = partial_round(&mut *cs, const_offset, elems, &params)?;
        const_offset += elems.len();
    }

    for _ in 0..params.full_rounds / 2 {
        elems = full_round(&mut *cs, const_offset, elems, &params)?;
        const_offset += elems.len();
    }

    Ok(elems[1].clone())
}

#[cfg(test)]
mod test;
