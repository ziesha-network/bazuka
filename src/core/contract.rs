use super::address::{Address, Signature};
use super::Money;
use crate::crypto::SignatureScheme;

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub struct ContractId {}

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
