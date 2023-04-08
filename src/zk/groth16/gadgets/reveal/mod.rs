use super::common::Number;
use super::poseidon::poseidon;
use crate::zk::groth16::gadgets::BellmanFr;
use crate::zk::ZkStateModel;
use bellman::{ConstraintSystem, SynthesisError};

#[derive(Clone)]
pub enum AllocatedState {
    Value(Number),
    Children(Vec<AllocatedState>),
}

pub fn reveal<CS: ConstraintSystem<BellmanFr>>(
    cs: &mut CS,
    state_model: &ZkStateModel,
    state: &AllocatedState,
) -> Result<Number, SynthesisError> {
    match state_model {
        ZkStateModel::Scalar => {
            if let AllocatedState::Value(v) = state {
                return Ok(v.clone());
            } else {
                panic!("Invalid state!");
            }
        }
        ZkStateModel::Struct { field_types } => {
            let mut vals = Vec::new();
            if let AllocatedState::Children(children) = state {
                for (field_type, field_value) in field_types.iter().zip(children.into_iter()) {
                    vals.push(reveal(&mut *cs, field_type, field_value)?);
                }
            } else {
                panic!("Invalid state!");
            }
            let ref_vals = vals.iter().map(|v| v).collect::<Vec<&Number>>();
            poseidon(&mut *cs, &ref_vals)
        }
        ZkStateModel::List {
            log4_size,
            item_type,
        } => {
            let mut leaves = Vec::new();
            if let AllocatedState::Children(children) = state {
                for i in 0..1 << (2 * log4_size) {
                    leaves.push(reveal(&mut *cs, item_type, &children[i])?);
                }
            } else {
                panic!("Invalid state!");
            }
            while leaves.len() != 1 {
                let mut new_leaves = Vec::new();
                for chunk in leaves.chunks(4) {
                    let hash = poseidon(&mut *cs, &[&chunk[0], &chunk[1], &chunk[2], &chunk[3]])?;
                    new_leaves.push(hash);
                }
                leaves = new_leaves;
            }
            Ok(leaves[0].clone())
        }
    }
}

#[cfg(test)]
mod test;
