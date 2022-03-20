use crate::core::Hash;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MerkleTree<H: Hash> {
    data: Vec<H::Output>,
}

fn merge_hash<H: Hash>(mut a: &H::Output, mut b: &H::Output) -> H::Output {
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
        i >> 1
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

    pub fn new(leaves: Vec<H::Output>) -> MerkleTree<H> {
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
