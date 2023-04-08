use super::*;
use crate::zk::groth16::gadgets::BellmanFr;
use bellman::gadgets::boolean::{AllocatedBit, Boolean};
use bellman::gadgets::num::AllocatedNum;
use bellman::{ConstraintSystem, LinearCombination, SynthesisError};
use ff::Field;
use std::ops::*;

#[derive(Clone)]
pub struct Number(pub LinearCombination<BellmanFr>, pub Option<BellmanFr>);
impl Number {
    pub fn get_lc(&self) -> &LinearCombination<BellmanFr> {
        &self.0
    }
    pub fn get_value(&self) -> Option<BellmanFr> {
        self.1
    }
    pub fn add_constant<CS: ConstraintSystem<BellmanFr>>(&mut self, num: BellmanFr) {
        self.0 = self.0.clone() + (num, CS::one());
        self.1 = self.1.map(|v| v + num);
    }
    pub fn add_num(&mut self, coeff: BellmanFr, num: &AllocatedNum<BellmanFr>) {
        self.0 = self.0.clone() + (coeff, num.get_variable());
        self.1 = if let Some(v) = self.1 {
            num.get_value().map(|n| n * coeff + v)
        } else {
            None
        };
    }
    pub fn constant<CS: ConstraintSystem<BellmanFr>>(v: BellmanFr) -> Number {
        Number(
            LinearCombination::<BellmanFr>::zero() + (v, CS::one()),
            Some(v),
        )
    }
    pub fn zero() -> Number {
        Number(
            LinearCombination::<BellmanFr>::zero(),
            Some(BellmanFr::zero()),
        )
    }
    pub fn one<CS: ConstraintSystem<BellmanFr>>() -> Number {
        Number(
            LinearCombination::<BellmanFr>::zero() + CS::one(),
            Some(BellmanFr::one()),
        )
    }
    pub fn mul<CS: ConstraintSystem<BellmanFr>>(
        &self,
        cs: &mut CS,
        other: &Number,
    ) -> Result<AllocatedNum<BellmanFr>, SynthesisError> {
        let result = AllocatedNum::alloc(&mut *cs, || {
            self.get_value()
                .zip(other.get_value())
                .map(|(a, b)| a * b)
                .ok_or(SynthesisError::AssignmentMissing)
        })?;
        cs.enforce(
            || "",
            |lc| lc + self.get_lc(),
            |lc| lc + other.get_lc(),
            |lc| lc + result.get_variable(),
        );
        Ok(result)
    }
    pub fn compress<CS: ConstraintSystem<BellmanFr>>(
        &self,
        cs: &mut CS,
    ) -> Result<AllocatedNum<BellmanFr>, SynthesisError> {
        self.mul::<CS>(cs, &Self::one::<CS>())
    }

    // Check if a number is zero, 2 constraints
    pub fn is_zero<CS: ConstraintSystem<BellmanFr>>(
        &self,
        cs: &mut CS,
    ) -> Result<Boolean, SynthesisError> {
        let is_zero =
            AllocatedBit::alloc(&mut *cs, self.get_value().map(|num| num.is_zero().into()))?;
        let inv = AllocatedNum::alloc(&mut *cs, || {
            self.get_value()
                .map(|num| {
                    if num.is_zero().into() {
                        BellmanFr::zero()
                    } else {
                        num.invert().unwrap()
                    }
                })
                .ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Alice claims "is_zero == 0", so "-num * inv == -1", so "inv" should be "1/num"
        // Calculating inv is only possible if num != 0

        cs.enforce(
            || "-num * inv == is_zero - 1",
            |lc| lc - self.get_lc(),
            |lc| lc + inv.get_variable(),
            |lc| lc + is_zero.get_variable() - CS::one(),
        );

        // Alice claims "is_zero == 1". Since "is_zero * num == 0", num can only be 0
        cs.enforce(
            || "is_zero * num == 0",
            |lc| lc + is_zero.get_variable(),
            |lc| lc + self.get_lc(),
            |lc| lc,
        );
        Ok(Boolean::Is(is_zero))
    }

    pub fn is_equal<CS: ConstraintSystem<BellmanFr>>(
        &self,
        cs: &mut CS,
        other: &Number,
    ) -> Result<Boolean, SynthesisError> {
        (self.clone() - other.clone()).is_zero(cs)
    }

    pub fn assert_equal<CS: ConstraintSystem<BellmanFr>>(&self, cs: &mut CS, other: &Number) {
        cs.enforce(
            || "",
            |lc| lc + self.get_lc(),
            |lc| lc + CS::one(),
            |lc| lc + other.get_lc(),
        );
    }

    pub fn assert_equal_if_enabled<CS: ConstraintSystem<BellmanFr>>(
        &self,
        cs: &mut CS,
        enabled: &Boolean,
        other: &Number,
    ) -> Result<(), SynthesisError> {
        match enabled {
            Boolean::Is(enabled) => {
                let enabled_value = enabled.get_value();
                let enabled_in_self = cs.alloc(
                    || "",
                    || {
                        enabled_value
                            .map(|e| {
                                if e {
                                    self.get_value()
                                } else {
                                    Some(BellmanFr::zero())
                                }
                            })
                            .unwrap_or(None)
                            .ok_or(SynthesisError::AssignmentMissing)
                    },
                )?;
                cs.enforce(
                    || "enabled * self == enabled_in_self",
                    |lc| lc + enabled.get_variable(),
                    |lc| lc + self.get_lc(),
                    |lc| lc + enabled_in_self,
                );
                cs.enforce(
                    || "enabled * other == enabled_in_self",
                    |lc| lc + enabled.get_variable(),
                    |lc| lc + other.get_lc(),
                    |lc| lc + enabled_in_self,
                );
            }
            Boolean::Not(_) => {
                unimplemented!();
            }
            Boolean::Constant(enabled) => {
                if *enabled {
                    self.assert_equal(&mut *cs, other);
                }
            }
        }
        Ok(())
    }
}

impl Add for Number {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(
            self.0 + &other.0,
            self.1.zip(other.1).map(|(slf, othr)| slf + othr),
        )
    }
}

impl Add<(BellmanFr, Number)> for Number {
    type Output = Number;

    fn add(self, other: (BellmanFr, Number)) -> Self {
        Self(
            self.0 + (other.0, &other.1 .0),
            self.1
                .zip(other.1 .1)
                .map(|(slf, othr)| slf + other.0 * othr),
        )
    }
}

impl Sub for Number {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self(
            self.0 - &other.0,
            self.1.zip(other.1).map(|(slf, othr)| slf - othr),
        )
    }
}

impl From<AllocatedNum<BellmanFr>> for Number {
    fn from(a: AllocatedNum<BellmanFr>) -> Self {
        Self(
            LinearCombination::<BellmanFr>::zero() + a.get_variable(),
            a.get_value(),
        )
    }
}

impl From<(BellmanFr, AllocatedNum<BellmanFr>)> for Number {
    fn from(a: (BellmanFr, AllocatedNum<BellmanFr>)) -> Self {
        Self(
            LinearCombination::<BellmanFr>::zero() + (a.0, a.1.get_variable()),
            a.1.get_value().map(|v| v * a.0),
        )
    }
}

impl From<AllocatedBit> for Number {
    fn from(a: AllocatedBit) -> Self {
        Self(
            LinearCombination::<BellmanFr>::zero() + a.get_variable(),
            a.get_value()
                .map(|b| BellmanFr::from(if b { 1 } else { 0 })),
        )
    }
}

impl From<UnsignedInteger> for Number {
    fn from(a: UnsignedInteger) -> Self {
        Self(
            LinearCombination::<BellmanFr>::zero() + a.get_lc(),
            a.get_value(),
        )
    }
}
