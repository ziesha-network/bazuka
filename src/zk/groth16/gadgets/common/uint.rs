use super::*;
use crate::zk::groth16::gadgets::BellmanFr;
use crate::zk::ZkScalar;
use bellman::gadgets::boolean::{AllocatedBit, Boolean};
use bellman::gadgets::num::AllocatedNum;
use bellman::{ConstraintSystem, LinearCombination, SynthesisError};
use ff::PrimeFieldBits;

#[derive(Clone)]
pub struct UnsignedInteger {
    bits: Vec<AllocatedBit>,
    num: Number,
}

impl UnsignedInteger {
    pub fn get_lc(&self) -> &LinearCombination<BellmanFr> {
        self.num.get_lc()
    }
    pub fn get_number(&self) -> &Number {
        &self.num
    }
    pub fn get_value(&self) -> Option<BellmanFr> {
        self.num.get_value()
    }
    pub fn bits(&self) -> &Vec<AllocatedBit> {
        &self.bits
    }
    pub fn num_bits(&self) -> usize {
        self.bits.len()
    }
    pub fn alloc<CS: ConstraintSystem<BellmanFr>>(
        cs: &mut CS,
        val: ZkScalar,
        bits: usize,
    ) -> Result<Self, SynthesisError> {
        let alloc = AllocatedNum::alloc(&mut *cs, || Ok(val.into()))?;
        Self::constrain(cs, alloc.into(), bits)
    }
    pub fn alloc_32<CS: ConstraintSystem<BellmanFr>>(
        cs: &mut CS,
        val: u32,
    ) -> Result<Self, SynthesisError> {
        Self::alloc(cs, ZkScalar::from(val as u64), 32)
    }
    pub fn alloc_64<CS: ConstraintSystem<BellmanFr>>(
        cs: &mut CS,
        val: u64,
    ) -> Result<Self, SynthesisError> {
        Self::alloc(cs, ZkScalar::from(val), 64)
    }
    pub fn constrain_strict<CS: ConstraintSystem<BellmanFr>>(
        cs: &mut CS,
        num: Number,
    ) -> Result<Self, SynthesisError> {
        let as_alloc = num.compress(&mut *cs)?;
        let mut bits = Vec::new();
        for bit in as_alloc.to_bits_le_strict(&mut *cs)? {
            if let Boolean::Is(bit) = bit {
                bits.push(bit);
            } else {
                return Err(SynthesisError::Unsatisfiable);
            }
        }
        Ok(UnsignedInteger { bits, num })
    }
    pub fn constrain<CS: ConstraintSystem<BellmanFr>>(
        cs: &mut CS,
        num: Number,
        num_bits: usize,
    ) -> Result<Self, SynthesisError> {
        let mut bits = Vec::new();
        let mut coeff = BellmanFr::one();
        let mut all = LinearCombination::<BellmanFr>::zero();
        let bit_vals: Option<Vec<bool>> = num
            .get_value()
            .map(|v| v.to_le_bits().iter().map(|b| *b).collect());
        for i in 0..num_bits {
            let bit = AllocatedBit::alloc(&mut *cs, bit_vals.as_ref().map(|b| b[i]))?;
            all = all + (coeff, bit.get_variable());
            bits.push(bit);
            coeff = coeff.double();
        }
        cs.enforce(
            || "check",
            |lc| lc + &all,
            |lc| lc + CS::one(),
            |lc| lc + num.get_lc(),
        );

        Ok(Self { num, bits })
    }

    // ~198 constraints
    pub fn lt<CS: ConstraintSystem<BellmanFr>>(
        &self,
        cs: &mut CS,
        other: &UnsignedInteger,
    ) -> Result<Boolean, SynthesisError> {
        assert_eq!(self.num_bits(), other.num_bits());
        let num_bits = self.num_bits();

        // Imagine a and b are two sigend (num_bits + 1) bits numbers
        let two_bits = BellmanFr::from(2).pow_vartime(&[num_bits as u64 + 1, 0, 0, 0]);
        let mut sub = self.num.clone() - other.num.clone();
        sub.add_constant::<CS>(two_bits);

        let sub_bits = UnsignedInteger::constrain(&mut *cs, sub, num_bits + 2)?;
        Ok(Boolean::Is(sub_bits.bits()[num_bits].clone()))
    }

    pub fn gt<CS: ConstraintSystem<BellmanFr>>(
        &self,
        cs: &mut CS,
        other: &UnsignedInteger,
    ) -> Result<Boolean, SynthesisError> {
        other.lt(cs, self)
    }

    pub fn lte<CS: ConstraintSystem<BellmanFr>>(
        &self,
        cs: &mut CS,
        other: &UnsignedInteger,
    ) -> Result<Boolean, SynthesisError> {
        Ok(self.gt(cs, other)?.not())
    }

    pub fn gte<CS: ConstraintSystem<BellmanFr>>(
        &self,
        cs: &mut CS,
        other: &UnsignedInteger,
    ) -> Result<Boolean, SynthesisError> {
        Ok(self.lt(cs, other)?.not())
    }
}
