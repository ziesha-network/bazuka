mod params;
pub use params::*;

use super::ZkScalar;
use ff::Field;
use std::ops::MulAssign;

#[derive(Debug, Clone, PartialEq)]
struct PoseidonState {
    constants_offset: usize,
    present_elements: u64,
    elements: Vec<ZkScalar>,
}

impl PoseidonState {
    fn arity(&self) -> usize {
        self.elements.len() - 1
    }

    pub fn new(elems: &[ZkScalar]) -> Self {
        let mut elements = elems.to_vec();
        elements.insert(0, ZkScalar::zero());
        Self {
            present_elements: 0u64,
            constants_offset: 0,
            elements,
        }
    }

    pub fn hash(&mut self) -> ZkScalar {
        let params = params_for_arity(self.arity());

        self.elements[0] = ZkScalar::from(self.present_elements);

        // 20 consts (4 * 5)
        for _ in 0..params.full_rounds / 2 {
            self.full_round();
        }

        // 300 consts (60 * 5)
        for _ in 0..params.partial_rounds {
            self.partial_round();
        }

        // 20 consts (4 * 50)
        for _ in 0..params.full_rounds / 2 {
            self.full_round();
        }

        self.elements[1]
    }

    pub fn full_round(&mut self) {
        // Every element of the merkle tree, plus the bitflag, is incremented by the round constants
        self.add_round_constants();

        // Apply the quintic S-Box to all elements
        self.elements.iter_mut().for_each(quintic_s_box);

        // Multiply the elements by the constant MDS matrix
        self.product_mds();
    }

    pub fn partial_round(&mut self) {
        // Every element of the merkle tree, plus the bitflag, is incremented by the round constants
        self.add_round_constants();

        // Apply the quintic S-Box to the bitflags element
        quintic_s_box(&mut self.elements[0]);

        // Multiply the elements by the constant MDS matrix
        self.product_mds();
    }

    fn add_round_constants(&mut self) {
        let params = params_for_arity(self.arity());
        let mut constants_offset = self.constants_offset;

        self.elements.iter_mut().for_each(|l| {
            *l += params.round_constants[constants_offset];
            constants_offset += 1;
        });

        self.constants_offset = constants_offset;
    }

    fn product_mds(&mut self) {
        let params = params_for_arity(self.arity());
        let mut result = vec![ZkScalar::from(0u64); self.elements.len()];

        for j in 0..self.elements.len() {
            for k in 0..self.elements.len() {
                result[j] += params.mds_constants[j][k] * self.elements[k];
            }
        }

        self.elements.copy_from_slice(&result);
    }
}

fn quintic_s_box(l: &mut ZkScalar) {
    let mut tmp = *l;
    tmp = tmp.square(); // l^2
    tmp = tmp.square(); // l^4
    l.mul_assign(&tmp); // l^5
}

pub fn poseidon(vals: &[ZkScalar]) -> ZkScalar {
    let mut h = PoseidonState::new(vals);
    h.hash()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ff::Field;

    #[test]
    fn test_hash_deterministic() {
        let mut h = PoseidonState::new(&[
            ZkScalar::one(),
            ZkScalar::one(),
            ZkScalar::one(),
            ZkScalar::one(),
        ]);

        let mut h2 = h.clone();
        let result = h.hash();

        assert_eq!(result, h2.hash());
    }

    #[test]
    fn test_hash_reflects_changes() {
        for arity in 1..MAX_ARITY + 1 {
            let mut vals = vec![ZkScalar::zero(); arity];
            let original = poseidon(&vals);
            for i in 0..vals.len() {
                vals[i] = ZkScalar::one();
                assert!(poseidon(&vals) != original);
            }
        }
    }
}
