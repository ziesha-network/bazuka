use super::*;

use crate::zk::groth16::gadgets::BellmanFr;
use bellman::gadgets::boolean::Boolean;
use bellman::{ConstraintSystem, SynthesisError};

pub fn extract_bool<CS: ConstraintSystem<BellmanFr>>(b: &Boolean) -> Number {
    match b.clone() {
        Boolean::Is(b) => b.into(),
        Boolean::Not(not_b) => Number::one::<CS>() - not_b.into(),
        Boolean::Constant(b_val) => {
            if b_val {
                Number::one::<CS>()
            } else {
                Number::zero()
            }
        }
    }
}

pub fn assert_true<CS: ConstraintSystem<BellmanFr>>(cs: &mut CS, b: &Boolean) {
    extract_bool::<CS>(b).assert_equal(cs, &Number::one::<CS>());
}

pub fn assert_true_if_enabled<CS: ConstraintSystem<BellmanFr>>(
    cs: &mut CS,
    enabled: &Boolean,
    cond: &Boolean,
) -> Result<(), SynthesisError> {
    extract_bool::<CS>(cond).assert_equal_if_enabled(cs, enabled, &Number::one::<CS>())
}

pub fn boolean_or<CS: ConstraintSystem<BellmanFr>>(
    cs: &mut CS,
    a: &Boolean,
    b: &Boolean,
) -> Result<Boolean, SynthesisError> {
    Ok(Boolean::and(&mut *cs, &a.not(), &b.not())?.not())
}
