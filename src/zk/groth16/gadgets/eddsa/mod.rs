use super::common::Number;
use super::{common, poseidon};
use crate::zk::groth16::gadgets::BellmanFr;

use crate::crypto::jubjub::{PointAffine, A, BASE_COFACTOR, D};

use bellman::gadgets::boolean::Boolean;
use bellman::gadgets::num::AllocatedNum;
use bellman::Variable;
use bellman::{ConstraintSystem, SynthesisError};
use std::ops::*;

#[derive(Clone)]
pub struct AllocatedPoint {
    pub x: AllocatedNum<BellmanFr>,
    pub y: AllocatedNum<BellmanFr>,
}

impl AllocatedPoint {
    pub fn get_variables(&self) -> (Variable, Variable) {
        (self.x.get_variable(), self.y.get_variable())
    }

    pub fn get_value(&self) -> Option<PointAffine> {
        self.x
            .get_value()
            .zip(self.y.get_value())
            .map(|(x, y)| PointAffine(x.into(), y.into()))
    }

    pub fn alloc<
        CS: ConstraintSystem<BellmanFr>,
        F: Fn() -> Result<PointAffine, SynthesisError>,
    >(
        cs: &mut CS,
        f: F,
    ) -> Result<AllocatedPoint, SynthesisError> {
        let x = AllocatedNum::<BellmanFr>::alloc(&mut *cs, || Ok(f()?.0.into()))?;
        let y = AllocatedNum::<BellmanFr>::alloc(&mut *cs, || Ok(f()?.1.into()))?;
        Ok(Self { x, y })
    }

    pub fn is_null<CS: ConstraintSystem<BellmanFr>>(
        &self,
        cs: &mut CS,
    ) -> Result<Boolean, SynthesisError> {
        let x_is_zero = Number::from(self.x.clone()).is_zero(&mut *cs)?;
        let y_is_zero = Number::from(self.y.clone()).is_zero(&mut *cs)?;
        Ok(Boolean::and(&mut *cs, &x_is_zero, &y_is_zero)?)
    }

    pub fn is_equal<CS: ConstraintSystem<BellmanFr>>(
        &self,
        cs: &mut CS,
        other: &AllocatedPoint,
    ) -> Result<Boolean, SynthesisError> {
        let xs_are_equal =
            Number::from(self.x.clone()).is_equal(&mut *cs, &Number::from(other.x.clone()))?;
        let ys_are_equal =
            Number::from(self.y.clone()).is_equal(&mut *cs, &Number::from(other.y.clone()))?;
        Ok(Boolean::and(&mut *cs, &xs_are_equal, &ys_are_equal)?)
    }

    pub fn assert_on_curve<CS: ConstraintSystem<BellmanFr>>(
        &self,
        cs: &mut CS,
        enabled: &Boolean,
    ) -> Result<(), SynthesisError> {
        let x2 = self.x.mul(&mut *cs, &self.x)?;
        let y2 = self.y.mul(&mut *cs, &self.y)?;
        let x2y2 = x2.mul(&mut *cs, &y2)?;
        let lhs = Number::from(y2) - Number::from(x2);
        let rhs = Number::from((BellmanFr::from(*D), x2y2)) + Number::one::<CS>();
        lhs.assert_equal_if_enabled(cs, enabled, &rhs)
    }

    pub fn add_const<CS: ConstraintSystem<BellmanFr>>(
        &self,
        cs: &mut CS,
        b: &PointAffine,
    ) -> Result<AllocatedPoint, SynthesisError> {
        let sum = AllocatedPoint::alloc(&mut *cs, || {
            self.get_value()
                .map(|a| {
                    if !a.is_on_curve() || !b.is_on_curve() {
                        // If invalid, do not need to calculate
                        Default::default()
                    } else {
                        let mut sum = a.clone();
                        sum.add_assign(&b);
                        sum
                    }
                })
                .ok_or(SynthesisError::AssignmentMissing)
        })?;

        let bx: BellmanFr = b.0.into();
        let by: BellmanFr = b.1.into();
        let curve_d_bx_by: BellmanFr = Into::<BellmanFr>::into(D.clone()) * bx * by;
        let common = self.x.mul(&mut *cs, &self.y)?; // * CURVE_D * bx * by

        cs.enforce(
            || "x_1 + x_2 == sum_x * (1 + common)",
            |lc| lc + CS::one() + (curve_d_bx_by, common.get_variable()),
            |lc| lc + sum.x.get_variable(),
            |lc| lc + (by, self.x.get_variable()) + (bx, self.y.get_variable()),
        );

        cs.enforce(
            || "y_1 - y_2 == sum_y * (1 - common)",
            |lc| lc + CS::one() - (curve_d_bx_by, common.get_variable()),
            |lc| lc + sum.y.get_variable(),
            |lc| {
                lc + (by, self.y.get_variable())
                    - (
                        Into::<BellmanFr>::into(A.clone()) * bx,
                        self.x.get_variable(),
                    )
            },
        );

        Ok(sum)
    }

    pub fn add<CS: ConstraintSystem<BellmanFr>>(
        &self,
        cs: &mut CS,
        other: &AllocatedPoint,
    ) -> Result<AllocatedPoint, SynthesisError> {
        let sum = AllocatedPoint::alloc(&mut *cs, || {
            self.get_value()
                .zip(other.get_value())
                .map(|(a, b)| {
                    if !a.is_on_curve() || !b.is_on_curve() {
                        // If invalid, do not need to calculate
                        Default::default()
                    } else {
                        let mut sum = a.clone();
                        sum.add_assign(&b);
                        sum
                    }
                })
                .ok_or(SynthesisError::AssignmentMissing)
        })?;

        let curve_d: BellmanFr = D.clone().into();
        let common = self
            .x
            .mul(&mut *cs, &other.x)?
            .mul(&mut *cs, &self.y)?
            .mul(&mut *cs, &other.y)?; // * CURVE_D

        let x_1 = self.x.mul(&mut *cs, &other.y)?;
        let x_2 = self.y.mul(&mut *cs, &other.x)?;
        cs.enforce(
            || "x_1 + x_2 == sum_x * (1 + common)",
            |lc| lc + CS::one() + (curve_d, common.get_variable()),
            |lc| lc + sum.x.get_variable(),
            |lc| lc + x_1.get_variable() + x_2.get_variable(),
        );

        let y_1 = self.y.mul(&mut *cs, &other.y)?;
        let y_2 = self.x.mul(&mut *cs, &other.x)?; // * CURVE_A
        cs.enforce(
            || "y_1 - y_2 == sum_y * (1 - common)",
            |lc| lc + CS::one() - (curve_d, common.get_variable()),
            |lc| lc + sum.y.get_variable(),
            |lc| lc + y_1.get_variable() - (A.clone().into(), y_2.get_variable()),
        );

        Ok(sum)
    }

    pub fn mul<CS: ConstraintSystem<BellmanFr>>(
        &self,
        cs: &mut CS,
        b: &AllocatedNum<BellmanFr>,
    ) -> Result<AllocatedPoint, SynthesisError> {
        let bits: Vec<Boolean> = b.to_bits_le_strict(&mut *cs)?.into_iter().rev().collect();
        let mut result = AllocatedPoint {
            x: common::mux(&mut *cs, &bits[0], &Number::zero(), &self.x.clone().into())?,
            y: common::mux(
                &mut *cs,
                &bits[0],
                &Number::constant::<CS>(BellmanFr::one()),
                &self.y.clone().into(),
            )?,
        };
        for bit in bits[1..].iter() {
            result = result.add(&mut *cs, &result)?;
            let result_plus_base = result.add(&mut *cs, self)?;
            let result_x =
                common::mux(&mut *cs, &bit, &result.x.into(), &result_plus_base.x.into())?;
            let result_y =
                common::mux(&mut *cs, &bit, &result.y.into(), &result_plus_base.y.into())?;
            result = AllocatedPoint {
                x: result_x,
                y: result_y,
            };
        }
        Ok(result)
    }
}

pub fn base_mul<CS: ConstraintSystem<BellmanFr>>(
    cs: &mut CS,
    base: &PointAffine,
    b: &AllocatedNum<BellmanFr>,
) -> Result<AllocatedPoint, SynthesisError> {
    let bits: Vec<Boolean> = b.to_bits_le_strict(&mut *cs)?.into_iter().rev().collect();
    let mut result = AllocatedPoint {
        x: common::mux(
            &mut *cs,
            &bits[0],
            &Number::zero(),
            &Number::constant::<CS>(base.0.into()),
        )?,
        y: common::mux(
            &mut *cs,
            &bits[0],
            &Number::constant::<CS>(BellmanFr::one()),
            &Number::constant::<CS>(base.1.into()),
        )?,
    };
    for bit in bits[1..].iter() {
        result = result.add(&mut *cs, &result)?;
        let result_plus_base = result.add_const(&mut *cs, base)?;
        let result_x = common::mux(&mut *cs, &bit, &result.x.into(), &result_plus_base.x.into())?;
        let result_y = common::mux(&mut *cs, &bit, &result.y.into(), &result_plus_base.y.into())?;
        result = AllocatedPoint {
            x: result_x,
            y: result_y,
        };
    }
    Ok(result)
}

// Mul by 8
fn mul_cofactor<CS: ConstraintSystem<BellmanFr>>(
    cs: &mut CS,
    point: &AllocatedPoint,
) -> Result<AllocatedPoint, SynthesisError> {
    let mut pnt = point.add(&mut *cs, point)?;
    pnt = pnt.add(&mut *cs, &pnt)?;
    pnt = pnt.add(&mut *cs, &pnt)?;
    Ok(pnt)
}

pub fn verify_eddsa<CS: ConstraintSystem<BellmanFr>>(
    cs: &mut CS,
    enabled: &Boolean,
    pk: &AllocatedPoint,
    msg: &Number,
    sig_r: &AllocatedPoint,
    sig_s: &AllocatedNum<BellmanFr>,
) -> Result<(), SynthesisError> {
    // h=H(R,A,M)
    let h = poseidon::poseidon(
        &mut *cs,
        &[
            &sig_r.x.clone().into(),
            &sig_r.y.clone().into(),
            &pk.x.clone().into(),
            &pk.y.clone().into(),
            msg,
        ],
    )?
    .compress(&mut *cs)?;

    let sb = base_mul(&mut *cs, &BASE_COFACTOR, sig_s)?;

    let mut r_plus_ha = pk.mul(&mut *cs, &h)?;
    r_plus_ha = r_plus_ha.add(&mut *cs, sig_r)?;
    r_plus_ha = mul_cofactor(&mut *cs, &r_plus_ha)?;

    Number::from(r_plus_ha.x).assert_equal_if_enabled(cs, enabled, &sb.x.into())?;
    Number::from(r_plus_ha.y).assert_equal_if_enabled(cs, enabled, &sb.y.into())?;

    Ok(())
}

#[cfg(test)]
mod test;
