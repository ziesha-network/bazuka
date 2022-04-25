use super::address::{Address, Signature};
use super::contract::{
    Circuit, CircuitProof, ContractCompressedState, ContractFullState, ContractId, ContractPayment,
};
use super::hash::Hash;
use super::Money;
use crate::crypto::SignatureScheme;

// A transaction could be as simple as sending some funds, or as complicated as
// creating a smart-contract.
#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub enum TransactionData<S: SignatureScheme> {
    RegularSend {
        dst: Address<S>,
        amount: Money,
    },
    RegisterValidator {
        vrf_stuff: u8,
        amount: Money,
    },

    // Create a Zero-Contract. The creator can consider multiple ways (Circuits) of updating
    // the state. But there should be only one circuit for entering and exiting the contract.
    CreateContract {
        deposit_withdraw_circuit: Circuit,
        update_circuits: Vec<Circuit>,
        initial_state: ContractFullState,
    },
    // Proof for DepositWithdrawCircuit(curr_state, next_state, hash(entries))
    DepositWithdraw {
        contract_id: ContractId,
        deposit_withdraws: Vec<ContractPayment<S>>,
        next_state: ContractCompressedState,
        proof: CircuitProof,
    },
    // Proof for UpdateCircuit[circuit_index](curr_state, next_state)
    Update {
        contract_id: ContractId,
        circuit_index: u32,
        next_state: ContractCompressedState,
        proof: CircuitProof,
    },
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub struct Transaction<S: SignatureScheme> {
    pub src: Address<S>,
    pub nonce: u32,
    pub data: TransactionData<S>,
    pub fee: Money,
    pub sig: Signature<S>,
}

impl<S: SignatureScheme> Transaction<S> {
    pub fn hash<H: Hash>(&self) -> H::Output {
        H::hash(&bincode::serialize(self).unwrap())
    }
    pub fn verify_signature(&self) -> bool {
        match &self.src {
            Address::<S>::Treasury => true,
            Address::<S>::PublicKey(pk) => match &self.sig {
                Signature::Unsigned => false,
                Signature::Signed(sig) => {
                    let mut unsigned = self.clone();
                    unsigned.sig = Signature::Unsigned;
                    let bytes = bincode::serialize(&unsigned).unwrap();
                    S::verify(&pk, &bytes, &sig)
                }
            },
        }
    }
}

impl<S: SignatureScheme + PartialEq> Eq for Transaction<S> {}
impl<S: SignatureScheme> std::hash::Hash for Transaction<S> {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        state.write(&bincode::serialize(self).unwrap());
        state.finish();
    }
}
