use super::address::{Address, Signature};
use super::Money;
use crate::crypto::SignatureScheme;

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub struct ContractId {}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub struct ContractState {
    pub state_hash: u8,    // State in compressed form
    pub state_size: usize, // Size of full state in bytes
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub enum PaymentDirection {
    Deposit,
    Withdraw,
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub struct ContractPayment<S: SignatureScheme> {
    initiator: Address<S>,
    contract_id: ContractId, // Makes sure the payment can only run on this contract.
    nonce: usize,            // Makes sure a contract payment cannot be replayed on this contract.
    amount: Money,
    fee: Money,
    direction: PaymentDirection,
    sig: Signature<S>,
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub struct CircuitProof {
    proof: u8,
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub struct Circuit {
    verifying_key: u8,
}
