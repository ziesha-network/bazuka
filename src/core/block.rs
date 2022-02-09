use crate::core::header::Header;
use primitive_types::U256;

#[derive(Clone, PartialEq, Eq)]
pub struct Block<C: PartialEq + Eq + Clone> {
    pub header: Header,
    pub data: Vec<C>,
}

impl<C> Block<C> {}

pub enum BlockId {
    Hash([u8; 32]),
    Num(U256),
}
