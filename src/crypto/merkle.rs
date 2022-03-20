use crate::core::{Hash, Transaction};

pub struct MerkleTree<H: Hash> {
    data: Vec<H::Output>,
}

impl<H: Hash> MerkleTree<H> {
    pub fn depth(&self) -> u32 {
        self.data.len().next_power_of_two().trailing_zeros() - 1
    }
    pub fn num_leaves(&self) -> usize {
        (self.data.len() + 1) >> 1
    }
    pub fn parent_map(&self, i: usize) -> usize {
        i >> 1
    }
    pub fn leaf_map(&self, i: usize) -> usize {
        let len = self.data.len();
        let dep = self.depth();
        let lower_start = (1 << dep) - 1;
        let lower_leaves = len - lower_start;
        let upper_start = (1 << (dep - 1)) - 1;
        let upper_offset = lower_leaves >> 1;
        return if lower_start + i < len {
            lower_start + i
        } else {
            upper_start - upper_offset + i
        };
    }
    pub fn get(&self, i: usize) -> H::Output {
        self.data[self.leaf_map(i)]
    }
    pub fn set(&mut self, i: usize, val: H::Output) {
        let mapped = self.leaf_map(i);
        self.data[mapped] = val;
    }
    fn merge(&self, a: &H::Output, _b: &H::Output) -> H::Output {
        // TODO: Merge hashes
        a.clone()
    }
    pub fn make_parents(&mut self) {
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
                let merged = self.merge(&self.data[i], &self.data[j]);
                let parent = self.parent_map(i);
                self.data[parent] = merged;
            }
        }
    }
    pub fn from_leaves(leaves: Vec<H::Output>) -> MerkleTree<H> {
        let data = vec![H::Output::default(); leaves.len() * 2 - 1];
        let mut tree = MerkleTree::new(data);
        for (i, val) in leaves.iter().enumerate() {
            tree.set(i, *val);
        }
        tree.make_parents();
        tree
    }

    pub fn new(data: Vec<H::Output>) -> MerkleTree<H> {
        MerkleTree::<H> { data }
    }
}
