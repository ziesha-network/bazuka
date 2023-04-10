use super::{DepositTransition, UpdateTransition, WithdrawTransition};

mod deposit_circuit;
mod update_circuit;
mod withdraw_circuit;
pub use deposit_circuit::*;
pub use update_circuit::*;
pub use withdraw_circuit::*;

pub trait MpnCircuit {
    fn empty(log4_tree_size: u8, log4_token_tree_size: u8, log4_batch_size: u8) -> Self;
}

#[cfg(test)]
mod test;
