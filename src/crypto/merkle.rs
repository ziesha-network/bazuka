use crate::core::hash::Hash;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MerkleTree<H: Hash> {
    data: Vec<H::Output>,
}

fn merge_hash<H: Hash>(a: &H::Output, b: &H::Output) -> H::Output {
    let mut inp = Vec::new();
    if a < b {
        inp.extend(a.as_ref());
        inp.extend(b.as_ref());
    } else {
        inp.extend(b.as_ref());
        inp.extend(a.as_ref());
    }
    H::hash(&inp)
}

impl<H: Hash> MerkleTree<H> {
    pub fn depth(&self) -> u32 {
        let len = self.data.len();
        if len == 1 {
            0
        } else {
            len.next_power_of_two().trailing_zeros() - 1
        }
    }

    pub fn num_leaves(&self) -> usize {
        (self.data.len() + 1) >> 1
    }

    fn parent_map(&self, i: usize) -> usize {
        (i - 1) >> 1
    }

    fn sibling_map(&self, i: usize) -> usize {
        if i % 2 == 0 {
            i - 1
        } else {
            i + 1
        }
    }

    fn leaf_map(&self, i: usize) -> usize {
        let len = self.data.len();
        let dep = self.depth();
        let lower_start = (1 << dep) - 1;
        let lower_leaves = len - lower_start;
        return if lower_start + i < len {
            lower_start + i
        } else {
            let upper_start = (1 << (dep - 1)) - 1;
            let upper_offset = lower_leaves >> 1;
            upper_start - upper_offset + i
        };
    }

    fn make_parents(&mut self) {
        let total = self.data.len();
        for d in (1..self.depth() + 1).rev() {
            let start = (1 << d) - 1;
            let len = 1 << d;
            for k in (0..len).step_by(2) {
                let i = start + k;
                let j = start + k + 1;
                if i >= total {
                    break;
                }
                let merged = merge_hash::<H>(&self.data[i], &self.data[j]);
                let parent = self.parent_map(i);
                self.data[parent] = merged;
            }
        }
    }

    pub fn root(&self) -> H::Output {
        self.data[0].clone()
    }

    pub fn prove(&self, leaf: usize) -> Vec<H::Output> {
        let mut proof = Vec::new();
        let mut ind = self.leaf_map(leaf);
        while ind != 0 {
            proof.push(self.data[self.sibling_map(ind)]);
            ind = self.parent_map(ind);
        }
        proof
    }

    pub fn new(leaves: Vec<H::Output>) -> MerkleTree<H> {
        if leaves.is_empty() {
            return MerkleTree::<H> {
                data: vec![H::Output::default()],
            };
        }
        let mut tree = MerkleTree::<H> {
            data: vec![H::Output::default(); leaves.len() * 2 - 1],
        };
        for (i, val) in leaves.iter().enumerate() {
            let mapped = tree.leaf_map(i);
            tree.data[mapped] = *val;
        }
        tree.make_parents();
        tree
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::hash::Sha3Hasher;

    #[test]
    fn test_merkle_proof() {
        let tree = MerkleTree::<Sha3Hasher>::new((0..10).map(|i| Sha3Hasher::hash(&[i])).collect());
        for i in 0..10 {
            let proof = tree.prove(i);
            let root = tree.root();
            let mut curr = Sha3Hasher::hash(&[i as u8]);
            for entry in proof {
                curr = merge_hash::<Sha3Hasher>(&curr, &entry);
            }
            assert_eq!(curr, root);
        }
    }

    #[test]
    fn test_calculation() {
        assert_eq!(MerkleTree::<Sha3Hasher>::new(Vec::new()).root(), [0u8; 32]);
        assert_eq!(
            MerkleTree::<Sha3Hasher>::new(vec![Sha3Hasher::hash(&[1])]).root(),
            [
                39, 103, 241, 92, 138, 242, 242, 199, 34, 93, 82, 115, 253, 214, 131, 237, 199, 20,
                17, 10, 152, 125, 16, 84, 105, 124, 52, 138, 237, 78, 108, 199
            ]
        );
        assert_eq!(
            MerkleTree::<Sha3Hasher>::new((2..4).map(|i| Sha3Hasher::hash(&[i])).collect()).root(),
            [
                147, 148, 62, 236, 12, 170, 57, 157, 174, 243, 124, 220, 81, 74, 187, 99, 252, 243,
                77, 85, 3, 93, 223, 166, 184, 93, 190, 149, 217, 73, 107, 7
            ]
        );
        assert_eq!(
            MerkleTree::<Sha3Hasher>::new((0..10).map(|i| Sha3Hasher::hash(&[i])).collect()).root(),
            [
                170, 152, 247, 242, 8, 76, 139, 70, 132, 168, 19, 116, 29, 8, 9, 42, 0, 85, 164,
                237, 192, 106, 123, 174, 180, 217, 32, 126, 18, 38, 210, 79
            ]
        );
        assert_eq!(
            MerkleTree::<Sha3Hasher>::new((0..16).map(|i| Sha3Hasher::hash(&[i])).collect()).root(),
            [
                205, 127, 119, 130, 101, 244, 191, 81, 239, 175, 89, 0, 91, 183, 65, 61, 170, 6,
                253, 155, 249, 90, 186, 20, 71, 105, 83, 24, 118, 68, 70, 119
            ]
        );
    }
}
