use std::fmt::Debug;
use std::hash::Hash;
use std::str::FromStr;

pub trait BlockT {}

pub trait HeaderT {
    type Number: Debug + Hash + Copy + FromStr;
    type Hash: Debug + Hash + Ord + Copy;
}
